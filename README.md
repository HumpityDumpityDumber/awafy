# AWAfy

<img src="https://nomnoml.com/image.svg?source=%5B%3Cstart%3Estart%5D%0A%0A%5Bstart%5D%20-%3E%0A%5B%20login%20%7C%0A%5Bgenerate%20device%20id%5D%20-%3E%0A%5Bpost%20%2Fv4%2Fcode%5D%20-%3E%0A%5Bpost%20%2Fv5%2Flogin%2Fcode%5D%20-%3E%0A%5Bcreate%20session%20from%20login%20info%5D%20-%3E%0A%5Bspit%20out%20refresh%20token%5D%0A%5D%0A%0A%5Bstart%5D%20-%3E%0A%5B%20Daemon%20%7C%0A%5B%20Queue%20Manager%20%7C%0A%5Bread%20AWAFY_TOKEN%20env%20var%5D%20-%3E%0A%5Bfetch%20queue%5D%20-%3E%0A%5Bconnect%20to%20websocket%20%7C%0A%5Bkeep%20track%20of%20lounge%20state%5D%0A%5Blisten%20for%20unpause%20and%20repause%5D%0A%5Bkeep%20lounge%20queue%20in%20sync%20with%20local%5D%0A%5D%0A%5D%0A%0A%5BPlayer%20%7C%0A%5Bread%20top%20song%20from%20queue%5D%20-%3E%0A%5Bstream%20song%20audio%20%26%20update%20plater%20state%20%7C%0A%5Brequest%20manifest%5D%20-%3E%0A%5Bstream%20audio%20from%20manifest%20data%5D%0A%5D%0A%5D%0A%5D%0A%0A%0A%5BModels%20%7C%0A%5Blounge%20%7C%20%0ALounge%20ID%0AName%0AState%0AQueue%0A%5D%0A%5BSession%20%7C%0AAccess%20token%0AToken%20expiry%0ARefresh%20token%0ADevice%20ID%0A%5D%0A%5BSong%20%7C%0AAlbum%20art%0ASong%20name%0AAlbum%0ATrack%20ID%0A%5D%0A%5BPlayerState%20%7C%0APlaying%0APosition%0ASong%0A%5D%0A%5BCode%20%7C%0ACode%0A%5D%0A%5D%0A%0A%0A%0A%0A%0A%0A%0A%0A%0A"/>

Current state:
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.64s
     Running `target\debug\awafy.exe login`
device id: Windows no work yet
#######
Code not authorized! Trying again in 3 seconds...
Code not authorized! Trying again in 3 seconds...
logged in as user knee!
This is your refresh token: ##########################
Set the environment variable AWAFY_TOKEN to this value before running awafy in daemon mode.
```

## Timeline

- [x] Finish device ID generation based on machine ID/GUID on Windows and Linux for API requests | 9/11
- [ ] Websocket connection to socket-gateway.awa.fm | 9/25
- [ ] Printing websocket messages to shell | 10/1
- [ ] Consume websocket messages and print formatted messages to shell | 10/8
- [ ] Keep lounge paused (listen for unpause messages and repause) | 10/18
- [ ] Full queue syncing | 11/18
---
- [ ] Player side of program | Remainder

## Dev Log


