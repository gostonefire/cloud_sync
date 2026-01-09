# Configure nginx
## Install nginx
* sudo apt update
* sudo apt install -y nginx
* sudo systemctl enable --now nginx

## Production
### Create the server block for sync.gridfire.org
* sudo nano /etc/nginx/sites-available/sync.gridfire.org.conf
```text
server {
    listen 80;
    listen [::]:80;
    server_name sync.gridfire.org;

    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name sync.gridfire.org;

    ssl_certificate     /etc/letsencrypt/live/gridfire.org-0001/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/gridfire.org-0001/privkey.pem;

    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;

    client_max_body_size 50m;

    # Standard reverse-proxy headers
    proxy_set_header Host              $host;
    proxy_set_header X-Real-IP         $remote_addr;
    proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;

    # =========================
    # ALLOWED PATHS ONLY
    # =========================

    # /grant → backend
    location = /grant {
        proxy_http_version 1.1;
        proxy_pass http://mygrid.gridfire.org:8000;
    }

    # /code → backend
    location = /code {
        proxy_http_version 1.1;
        proxy_pass http://mygrid.gridfire.org:8000;
    }

    # Optional: allow subpaths if needed
    # location ^~ /grant/ {
    #     proxy_http_version 1.1;
    #     proxy_pass http://mygrid.gridfire.org:8000;
    # }
    #
    # location ^~ /code/ {
    #     proxy_http_version 1.1;
    #     proxy_pass http://mygrid.gridfire.org:8000;
    # }

    # =========================
    # DENY EVERYTHING ELSE
    # =========================
    location / {
        return 404;
        # or: return 403;
    }
}
```
### Enable the site and reload Nginx
* sudo ln -s /etc/nginx/sites-available/sync.gridfire.org.conf /etc/nginx/sites-enabled/
* sudo rm -f /etc/nginx/sites-enabled/default
* sudo nginx -t
* sudo systemctl reload nginx
