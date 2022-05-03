# System Updater Testing

Testing requires having another staging repo ready that can be added/removed to simulate updates becoming available.

Watch the following logs while testing:

```
sudo journalctl -u com.system76.SystemUpdater.service -f
journalctl --user -u com.system76.SystemUpdater.Local.service -f
```

## Update Installation

- Set `Automatic Updates` to enabled.
- Make updates available.
- [ ] Set `Schedule Automatic Updates` to one minute after the current time; wait and verify that updates run.

## Update Notification

- Make updates available.
- [ ] Set `Schedule Automatic Updates` to one minute after the current time, then turn `Automatic Updates` off; confirm updates do not run.
- [ ] Edit or remove `~/.cache/pop-system-updater/cache.ron`, restart the `--user` service, and confirm a notification is displayed.
- [ ] Click the notification and confirm that the Pop!\_Shop opens to the Installed tab.
