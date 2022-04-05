# stm32f429-smoltcp-rtic

Example RTIC & smoltcp app.

```
cargo run --release

(HOST) INFO  flashing program (101.63 KiB)
(HOST) INFO  success!
────────────────────────────────────────────────────────────────────────────────
INFO - Starting
INFO - Setup: ETH
INFO - Setup: PHY
DEBUG - Reset PHY
DEBUG - Reset complete Bmcr { collision_test: false, force_fd: false, restart_an: false, isolate: false, power_down: false, an_enable: true, force_100: true, loopback: false, soft_reset: false }
DEBUG - Setup PHY
DEBUG - Bmcr { collision_test: false, force_fd: true, restart_an: false, isolate: false, power_down: false, an_enable: true, force_100: true, loopback: false, soft_reset: false }
INFO - Setup: waiting for link
INFO - Setup: TCP/IP
INFO - IP: 192.168.1.39 MAC: 02-00-05-06-07-08
INFO - Setup: net clock timer
INFO - Setup: net link check timer
INFO - Setup: net poll timer
INFO - Initialized
INFO - Binding to UDP port 12345
INFO - Got 5 bytes from 192.168.1.abc:36291
INFO - Got 5 bytes from 192.168.1.abc:45166
────────────────────────────────────────────────────────────────────────────────
```
