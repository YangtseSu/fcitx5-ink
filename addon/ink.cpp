// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

// fcitx5-ink: a standalone handwriting input method for fcitx5.
//
// The addon owns the focused input context and a small user-private socket.
// The GTK4 panel draws ink, recognizes it with the vendored handwriting engine,
// hides itself, and asks this addon to commit the chosen text. No msime host
// header or platform code is compiled here.

#include <fcitx/addonfactory.h>
#include <fcitx/addonmanager.h>
#include <fcitx/event.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/instance.h>
#include <fcitx-utils/event.h>
#include <fcitx-utils/eventloopinterface.h>
#include <fcitx-utils/handlertable.h>
#include <fcitx-utils/log.h>
#include <nlohmann/json.hpp>

#include <cerrno>
#include <chrono>
#include <csignal>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <memory>
#include <optional>
#include <poll.h>
#include <spawn.h>
#include <string>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <unistd.h>
#include <utility>
#include <vector>

namespace fcitx5_ink {

namespace {

constexpr size_t kLineLimit = 16 * 1024;
constexpr size_t kTextLimit = 4096;
constexpr size_t kPendingLimit = 16;
constexpr auto kPendingWait = std::chrono::milliseconds(700);
constexpr auto kPumpInterval = std::chrono::milliseconds(50);

std::string socket_path() {
  const char *runtime = std::getenv("XDG_RUNTIME_DIR");
  if (!runtime || runtime[0] != '/')
    return {};
  return std::string(runtime) + "/fcitx5-ink/panel-input.sock";
}

const char *panel_program() {
  const char *configured = std::getenv("FCITX5_INK_PANEL");
  if (configured && *configured)
    return configured;
  return FCITX5_INK_PANEL_PATH;
}

fcitx::KeyList default_hotkey() {
  if (const char *configured = std::getenv("FCITX5_INK_HOTKEY");
      configured && *configured) {
    fcitx::Key key(configured);
    if (key.isValid())
      return {key};
  }
  return {fcitx::Key("Control+Alt+H")};
}

bool restricted(fcitx::InputContext *ic) {
  return !ic || ic->capabilityFlags().testAny(fcitx::CapabilityFlags{
                      fcitx::CapabilityFlag::Password,
                      fcitx::CapabilityFlag::Disable});
}

bool valid_text(const std::string &text) {
  if (text.empty() || text.size() > kTextLimit)
    return false;
  for (const unsigned char byte : text) {
    if (byte < 0x20 || byte == 0x7f)
      return false;
  }
  return true;
}

class PanelSocket {
public:
  PanelSocket() = default;
  PanelSocket(const PanelSocket &) = delete;
  PanelSocket &operator=(const PanelSocket &) = delete;
  ~PanelSocket() { close(); }

  int fd() const { return fd_; }

  bool open(const std::string &path) {
    if (fd_ >= 0)
      return true;
    if (path.empty() || path.size() >= sizeof(sockaddr_un{}.sun_path))
      return false;

    const auto directory = std::filesystem::path(path).parent_path();
    std::error_code error;
    std::filesystem::create_directories(directory, error);
    if (error || ::chmod(directory.c_str(), 0700) != 0)
      return false;
    struct stat directory_info {};
    if (::lstat(directory.c_str(), &directory_info) != 0 ||
        !S_ISDIR(directory_info.st_mode) || directory_info.st_uid != ::getuid())
      return false;

    sockaddr_un address{};
    address.sun_family = AF_UNIX;
    std::memcpy(address.sun_path, path.c_str(), path.size() + 1);

    struct stat existing {};
    if (::lstat(path.c_str(), &existing) == 0) {
      if (!S_ISSOCK(existing.st_mode))
        return false;
      const int probe =
          ::socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0);
      if (probe < 0)
        return false;
      const bool live =
          ::connect(probe, reinterpret_cast<const sockaddr *>(&address),
                    sizeof(address)) == 0;
      const int connect_error = errno;
      ::close(probe);
      if (live || connect_error != ECONNREFUSED)
        return false;
      ::unlink(path.c_str());
    }

    const int fd =
        ::socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0);
    if (fd < 0)
      return false;
    if (::bind(fd, reinterpret_cast<const sockaddr *>(&address),
               sizeof(address)) != 0 ||
        ::chmod(path.c_str(), 0600) != 0 || ::listen(fd, 8) != 0) {
      ::close(fd);
      return false;
    }
    fd_ = fd;
    path_ = path;
    return true;
  }

  std::optional<std::pair<int, std::string>> accept_request() {
    if (fd_ < 0)
      return std::nullopt;
    const int connection = ::accept4(fd_, nullptr, nullptr, SOCK_CLOEXEC);
    if (connection < 0)
      return std::nullopt;

    ucred peer{};
    socklen_t size = sizeof(peer);
    if (::getsockopt(connection, SOL_SOCKET, SO_PEERCRED, &peer, &size) != 0 ||
        peer.uid != ::getuid()) {
      ::close(connection);
      return std::nullopt;
    }

    std::string line;
    char buffer[1024];
    const auto deadline =
        std::chrono::steady_clock::now() + std::chrono::milliseconds(200);
    while (std::chrono::steady_clock::now() < deadline &&
           line.size() <= kLineLimit) {
      pollfd ready{connection, POLLIN, 0};
      if (::poll(&ready, 1, 20) <= 0)
        continue;
      const auto count = ::recv(connection, buffer, sizeof(buffer), MSG_DONTWAIT);
      if (count <= 0)
        break;
      line.append(buffer, static_cast<size_t>(count));
      if (const auto end = line.find('\n'); end != std::string::npos) {
        line.resize(end);
        return std::make_pair(connection, std::move(line));
      }
    }
    ::close(connection);
    return std::nullopt;
  }

  static void reply_and_close(int connection, const std::string &reply) {
    const std::string line = reply + "\n";
    size_t offset = 0;
    while (offset < line.size()) {
      const auto count =
          ::send(connection, line.data() + offset, line.size() - offset,
                 MSG_NOSIGNAL | MSG_DONTWAIT);
      if (count <= 0)
        break;
      offset += static_cast<size_t>(count);
    }
    ::close(connection);
  }

  void close() {
    if (fd_ < 0)
      return;
    ::close(fd_);
    fd_ = -1;
    if (!path_.empty())
      ::unlink(path_.c_str());
  }

private:
  int fd_ = -1;
  std::string path_;
};

} // namespace

class InkEngine final : public fcitx::InputMethodEngineV2 {
public:
  explicit InkEngine(fcitx::Instance *instance)
      : instance_(instance), hotkey_(default_hotkey()) {
    const auto path = socket_path();
    if (path.empty() || !socket_.open(path)) {
      FCITX_WARN() << "ink: cannot open panel socket " << path;
    } else {
      io_ = instance_->eventLoop().addIOEvent(
          socket_.fd(), fcitx::IOEventFlag::In,
          [this](fcitx::EventSourceIO *, int, fcitx::IOEventFlags) {
            accept_request();
            pump();
            return true;
          });
    }

    hotkey_watch_ = instance_->watchEvent(
        fcitx::EventType::InputContextKeyEvent,
        fcitx::EventWatcherPhase::PreInputMethod, [this](fcitx::Event &event) {
          auto &key = static_cast<fcitx::KeyEvent &>(event);
          if (key.isRelease() || !key.key().checkKeyList(hotkey_))
            return;
          auto *ic = key.inputContext();
          if (restricted(ic))
            return;
          key.filterAndAccept();
          if (panel_pid_ > 0 && target_ == ic) {
            close_panel();
            target_ = nullptr;
          } else {
            target_ = ic;
            launch_panel();
          }
        });

    focus_watch_ = instance_->watchEvent(
        fcitx::EventType::InputContextFocusIn,
        fcitx::EventWatcherPhase::PreInputMethod, [this](fcitx::Event &event) {
          auto *ic = static_cast<fcitx::InputContextEvent &>(event).inputContext();
          if (target_ && target_ != ic && panel_pid_ > 0)
            close_panel();
          if (instance_->inputMethodEngine(ic) == this)
            target_ = ic;
        });

    destroy_watch_ = instance_->watchEvent(
        fcitx::EventType::InputContextDestroyed,
        fcitx::EventWatcherPhase::PreInputMethod, [this](fcitx::Event &event) {
          auto *ic = static_cast<fcitx::InputContextEvent &>(event).inputContext();
          if (target_ == ic) {
            close_panel();
            target_ = nullptr;
          }
        });
  }

  void activate(const fcitx::InputMethodEntry &,
                fcitx::InputContextEvent &event) override {
    target_ = event.inputContext();
    if (!restricted(target_))
      launch_panel();
  }

  void deactivate(const fcitx::InputMethodEntry &,
                  fcitx::InputContextEvent &event) override {
    // The panel takes focus while it is open. Keep it alive on that focus-out;
    // an actual input method switch closes it.
    if (event.type() == fcitx::EventType::InputContextFocusOut)
      return;
    close_panel();
    target_ = nullptr;
  }

  void reset(const fcitx::InputMethodEntry &,
             fcitx::InputContextEvent &) override {
    close_panel();
    target_ = nullptr;
  }

  void keyEvent(const fcitx::InputMethodEntry &, fcitx::KeyEvent &) override {
    // The global hotkey watcher and the panel own the interaction. With the
    // panel closed, ordinary keys pass through unchanged.
  }

  std::vector<fcitx::InputMethodEntry> listInputMethods() override {
    std::vector<fcitx::InputMethodEntry> entries;
    entries.emplace_back("ink", "Ink", "zh_CN", "ink");
    entries.back().setNativeName("墨笔").setLabel("墨");
    return entries;
  }

private:
  struct Pending {
    int fd;
    std::string text;
    std::chrono::steady_clock::time_point deadline;
  };

  void accept_request() {
    auto accepted = socket_.accept_request();
    if (!accepted)
      return;
    const int fd = accepted->first;
    try {
      const auto request = nlohmann::json::parse(accepted->second);
      if (!request.is_object() || request.value("op", "") != "text") {
        PanelSocket::reply_and_close(fd, R"({"ok":false,"error":"invalid"})");
        return;
      }
      const auto text = request.value("text", "");
      if (!valid_text(text)) {
        PanelSocket::reply_and_close(fd, R"({"ok":false,"error":"invalid"})");
        return;
      }
      submit_text(fd, text);
    } catch (...) {
      PanelSocket::reply_and_close(fd, R"({"ok":false,"error":"invalid"})");
    }
  }

  void submit_text(int fd, const std::string &text) {
    if (target_ && target_->hasFocus() && !restricted(target_)) {
      target_->commitString(text);
      PanelSocket::reply_and_close(fd, R"({"ok":true})");
      return;
    }
    if (pending_.size() >= kPendingLimit) {
      PanelSocket::reply_and_close(fd, R"({"ok":false,"error":"no_focus"})");
      return;
    }
    pending_.push_back(
        Pending{fd, text, std::chrono::steady_clock::now() + kPendingWait});
    pump();
  }

  void pump() {
    const auto now = std::chrono::steady_clock::now();
    for (auto entry = pending_.begin(); entry != pending_.end();) {
      if (target_ && target_->hasFocus() && !restricted(target_)) {
        target_->commitString(entry->text);
        PanelSocket::reply_and_close(entry->fd, R"({"ok":true})");
        entry = pending_.erase(entry);
        continue;
      }
      if (now >= entry->deadline) {
        PanelSocket::reply_and_close(entry->fd,
                                     R"({"ok":false,"error":"no_focus"})");
        entry = pending_.erase(entry);
        continue;
      }
      ++entry;
    }
    if (pending_.empty()) {
      if (pump_timer_)
        pump_timer_->setEnabled(false);
      return;
    }
    if (!pump_timer_) {
      pump_timer_ = instance_->eventLoop().addTimeEvent(
          CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 50000, 50000,
          [this](fcitx::EventSourceTime *, uint64_t) {
            pump();
            return true;
          });
      pump_timer_->setOneShot();
    } else if (!pump_timer_->isEnabled()) {
      pump_timer_->setNextInterval(50000);
      pump_timer_->setOneShot();
      pump_timer_->setEnabled(true);
    }
  }

  void launch_panel() {
    if (panel_pid_ > 0) {
      const pid_t result = ::waitpid(panel_pid_, nullptr, WNOHANG);
      if (result == 0)
        return;
      panel_pid_ = 0;
    }
    const auto socket = socket_path();
    if (socket.empty())
      return;
    const char *program = panel_program();
    char *arguments[] = {
        const_cast<char *>(program),
        const_cast<char *>("--socket"),
        const_cast<char *>(socket.c_str()),
        nullptr,
    };
    pid_t child = 0;
    if (::posix_spawnp(&child, program, nullptr, nullptr, arguments, ::environ) ==
        0) {
      panel_pid_ = child;
      FCITX_INFO() << "ink: launched " << program;
    } else {
      FCITX_WARN() << "ink: failed to launch " << program;
    }
  }

  void close_panel() {
    if (panel_pid_ <= 0)
      return;
    ::kill(panel_pid_, SIGTERM);
    for (int step = 0; step < 20; ++step) {
      const pid_t result = ::waitpid(panel_pid_, nullptr, WNOHANG);
      if (result == panel_pid_ || (result < 0 && errno == ECHILD)) {
        panel_pid_ = 0;
        return;
      }
      ::usleep(10000);
    }
    panel_pid_ = 0;
  }

  fcitx::Instance *instance_;
  fcitx::KeyList hotkey_;
  PanelSocket socket_;
  std::vector<Pending> pending_;
  std::unique_ptr<fcitx::EventSourceIO> io_;
  std::unique_ptr<fcitx::EventSourceTime> pump_timer_;
  std::unique_ptr<fcitx::HandlerTableEntry<fcitx::EventHandler>> hotkey_watch_;
  std::unique_ptr<fcitx::HandlerTableEntry<fcitx::EventHandler>> focus_watch_;
  std::unique_ptr<fcitx::HandlerTableEntry<fcitx::EventHandler>> destroy_watch_;
  fcitx::InputContext *target_ = nullptr;
  pid_t panel_pid_ = 0;
};

class InkFactory final : public fcitx::AddonFactory {
public:
  fcitx::AddonInstance *create(fcitx::AddonManager *manager) override {
    return new InkEngine(manager->instance());
  }
};

} // namespace fcitx5_ink

FCITX_ADDON_FACTORY(fcitx5_ink::InkFactory)
