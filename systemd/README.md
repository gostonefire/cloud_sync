# Configure Systemd
* Check paths in `start.sh` and `cloudsync.service`
* Copy `cloudsync.service` to `/lib/systemd/system/`
* Run `sudo systemctl enable cloudsync.service`
* Run `sudo systemctl start cloudsync.service`
* Check status by running `sudo systemctl status cloudsync.service`

Output should be something like:
```
● cloudsync.service - Cloud sync between OneDrv and AWS S3
     Loaded: loaded (/lib/systemd/system/cloudsync.service; enabled; preset: enabled)
     Active: active (running) since Sat 2025-07-26 15:11:37 CEST; 1min 2s ago
   Main PID: 106258 (bash)
      Tasks: 7 (limit: 9573)
        CPU: 111ms
     CGroup: /system.slice/cloudsync.service
             ├─106258 /bin/bash /home/petste/CloudSync/start.sh
             └─106259 /home/petste/CloudSync/cloud_sync

Jul 26 15:11:37 mygrid systemd[1]: Started cloudsync.service - Cloud sync between OneDrv and AWS S3.
```

If the application for some reason prints anything to stdout/stderr, such in case of a panic,
the log for that can be found by using `journalctl -u cloudsync.service`.