// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later
#include <gtk/gtk.h>
#include <stdio.h>

static void changed(GtkEditable *editable, gpointer unused) {
  (void)unused;
  printf("TEXT:%s\n", gtk_entry_get_text(GTK_ENTRY(editable)));
  fflush(stdout);
}

int main(int argc, char **argv) {
  gtk_init(&argc, &argv);
  GtkWidget *window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
  gtk_window_set_title(GTK_WINDOW(window), "fcitx5-ink-test");
  gtk_window_set_default_size(GTK_WINDOW(window), 320, 80);
  GtkWidget *entry = gtk_entry_new();
  gtk_container_add(GTK_CONTAINER(window), entry);
  g_signal_connect(entry, "changed", G_CALLBACK(changed), NULL);
  gtk_widget_show_all(window);
  gtk_widget_grab_focus(entry);
  gtk_main();
  return 0;
}
