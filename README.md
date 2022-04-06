# stm32f429-smoltcp-mqtt-rtic

Example MQTT client, smoltcp IP stack, running on RTIC.

```
export MAC_ADDRESS="02:00:00:03:02:00"
export IP_ADDRESS="a.b.c.d"
export BROKER_IP_ADDRESS="a.b.c.e"

cargo run --release

(HOST) INFO  flashing program (184.70 KiB)
(HOST) INFO  success!
────────────────────────────────────────────────────────────────────────────────
INFO - mqtt-rtic version 0.1.0
INFO - MAC address: 02-03-04-05-06-07
INFO - IP address: a.b.c.d
INFO - Broker IP address: a.b.c.d
INFO - --- Starting hardware setup
INFO - Setup GPIO
INFO - Setup Ethernet
INFO - Setup phy
INFO - Waiting for link
INFO - Setup TCP/IP
INFO - Setup SysTick
INFO - Setup network
INFO - --- Hardware setup done
INFO - MQTT connected, subscribing to settings
INFO - Settings update: `led`
────────────────────────────────────────────────────────────────────────────────
```
