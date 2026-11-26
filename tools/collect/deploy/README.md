# Collector deployment

Target: Debian 13, nginx, certbot, a dedicated `ink` user, `https://886945.xyz`.

Replace `<VPS>` with the SSH destination and `<EMAIL>` with the certificate
contact address. Do not put the config secret in the repository.

## 1. Packages and user

```sh
apt-get update
apt-get install -y nginx certbot git rsync ufw
useradd --system --create-home --home-dir /srv/ink --shell /usr/sbin/nologin ink
install -d -o ink -g ink -m 750 /srv/ink/app /srv/ink/targets /srv/ink/data /srv/ink/backup
install -d -o root -g root -m 755 /srv/ink/web
install -d -o root -g root -m 755 /var/www/certbot
```

## 2. Files

From the repository root:

```sh
rsync -av tools/collect/server.py tools/collect/admin.py root@<VPS>:/srv/ink/app/
rsync -av tools/collect/web/ root@<VPS>:/srv/ink/web/
rsync -av --delete tools/collect/targets/train/ root@<VPS>:/srv/ink/targets/
rsync -av tools/collect/deploy/ root@<VPS>:/tmp/ink-deploy/
ssh root@<VPS> '
  chown -R root:root /srv/ink/web &&
  chmod -R a+rX /srv/ink/web &&
  chown ink:ink /srv/ink/app /srv/ink/targets /srv/ink/data /srv/ink/backup &&
  chmod 750 /srv/ink/app /srv/ink/targets /srv/ink/data /srv/ink/backup
'
```

## 3. Config and service

```sh
install -d -o root -g ink -m 750 /etc/ink
install -m 640 -o root -g ink /tmp/ink-deploy/collect.example.json /etc/ink/collect.json
secret=$(openssl rand -hex 32)
sed -i "s/REPLACE_WITH_RANDOM_SECRET/$secret/" /etc/ink/collect.json

install -m 644 /tmp/ink-deploy/ink-collect.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now ink-collect
systemctl status --no-pager ink-collect
curl -s http://127.0.0.1:8081/api/health
```

## 4. TLS

Install the HTTP-only vhost first, because the final one references a
certificate that does not exist yet:

```sh
rm -f /etc/nginx/sites-enabled/default
install -m 644 /tmp/ink-deploy/nginx-ink-http.conf /etc/nginx/sites-available/ink.conf
ln -sf /etc/nginx/sites-available/ink.conf /etc/nginx/sites-enabled/ink.conf
nginx -t && systemctl reload nginx

certbot certonly --webroot -w /var/www/certbot \
  -d 886945.xyz --agree-tos --non-interactive -m <EMAIL> \
  --deploy-hook "systemctl reload nginx"

install -m 644 /tmp/ink-deploy/nginx-ink.conf /etc/nginx/sites-available/ink.conf
nginx -t && systemctl reload nginx
```

The certificate is renewed by `certbot.timer`; the deploy hook reloads nginx.

## 5. Firewall

```sh
ufw allow 22/tcp
ufw allow 80/tcp
ufw allow 443/tcp
ufw --force enable
ufw status
```

## 6. Backups

```sh
install -m 644 /tmp/ink-deploy/ink-backup.service /tmp/ink-deploy/ink-backup.timer /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now ink-backup.timer
systemctl start ink-backup.service
```

Pull backups off the server regularly:

```sh
rsync -av root@<VPS>:/srv/ink/backup/ ./ink-backup/
```

## 7. Evaluation mode (maintainer)

The evaluation pool is a separate service and database; the public training
flow is untouched.

```sh
install -d -o ink -g ink -m 750 /srv/ink/eval-targets /srv/ink/eval-data
rsync -av --delete tools/collect/targets/eval/ root@<VPS>:/srv/ink/eval-targets/
install -m 640 -o root -g ink /tmp/ink-deploy/eval-collect.example.json /etc/ink/eval-collect.json
token=$(openssl rand -hex 16)
secret=$(openssl rand -hex 32)
sed -i "s/REPLACE_WITH_RANDOM_SECRET/$secret/; s/REPLACE_WITH_EVAL_TOKEN/$token/" /etc/ink/eval-collect.json
install -m 644 /tmp/ink-deploy/ink-eval.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now ink-eval
nginx -t && systemctl reload nginx
```

Set `eval_token` in `/etc/ink/eval-collect.json`; open
`https://<DOMAIN>/eval`, enter the token, and write the 300-character pool.
The `ink_eval_` cookies keep the evaluation writer separate from the training
writer. Export with the eval config:

```sh
python3 /srv/ink/app/admin.py --config /etc/ink/eval-collect.json \
  export --out /tmp/ink-eval-export
```

The nginx `/eval-api/` location proxies to `127.0.0.1:8082`; `/eval` serves the
same page with `X-Robots-Tag: noindex`. The eval token is a credential: keep it
out of the repository.

## 8. Checks

```sh
curl -I http://886945.xyz/
curl -I https://886945.xyz/
curl -s  https://886945.xyz/api/health
curl -s "https://886945.xyz/api/targets?lang=zh_CN" | head -c 200; echo
curl -s  https://886945.xyz/api/stats
curl -I https://886945.xyz/does-not-exist
```

The last command must return the styled 404 page, not an nginx default page.
There must be no access log containing client IPs; the service logs accepted
sample IDs without addresses.
