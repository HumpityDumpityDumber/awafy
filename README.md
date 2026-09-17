# AWAfy
AWAfy is a music daemon that streams music from AWA and uses AWA Lounge to allow for an online server synced queue experience. Made to be controlled through the web or app lounge interface.

Current state:
```
[src\https.rs:35:13] &session.access_token.as_str() = "REDACTED"
[src\https.rs:35:13] &session.access_token.as_str() = "REDACTED"
[src\https.rs:155:9] &response["playerState"] = Object {}
[src\commands\daemon.rs:33:5] &lounge.name = "odontoid 螺旋 [ねじ]"
[src\commands\daemon.rs:34:5] &lounge.id = "6aa939244114dd4c0400b97e"
User joined: {
  "id": "##########################",
  "isEssentialLiver": false,
  "isPickedOutLiver": false,
  "name": "knee",
  "socketId": "IEKjRG8l6TBmtbivAAou",
  "updatedAt": 1789475106,
  "uploadedAt": 1750654445
}
[src\commands\daemon.rs:87:41] "Unrecognized message string:" = "Unrecognized message string:"
[src\commands\daemon.rs:87:41] other = Some(
    "receive:owner:user_list:update:v1",
)
User joined: {
  "id": "##########################",
  "isEssentialLiver": false,
  "isPickedOutLiver": false,
  "name": "knee",
  "socketId": "6B3QHV3DidASZ1d4ADIv",
  "updatedAt": 1789475106,
  "uploadedAt": 1750654445
}
User joined: {
  "id": "##########################",
  "isEssentialLiver": false,
  "isPickedOutLiver": false,
  "name": "knee",
  "socketId": "gb6v0MesfkSM7IU2AAF_",
  "updatedAt": 1789475106,
  "uploadedAt": 1750654445
}
```

## Timeline
- [x] Login flow | ✅
- [x] Finish device ID generation based on machine ID/GUID on Windows and Linux for API requests | 9/11
- [x] SocketIO connection to socket-gateway.awa.fm | 9/11
- [x] Printing websocket messages to shell | 9/14
- [ ] Keep lounge paused | 9/20
- [ ] Parsing of song manifest | 9/27
- [ ] Requesting and decoding of music chunks | 10/7
- [ ] Finished audio playback | 10/15
- [ ] Fully integrate the two halves of the program | 10/22
- [ ] Automatic joining of lounges with specific key words for automatic startup | 10/29
Testing my program on the raspberry pi?
- [ ] Windows SMTC | 11/6
- [ ] Linux MPRIS | 11/11
- [ ] Integrate favoriting a song into both MPRIS and SMTC | 11/18
- [ ] Discord rpc integration | 11/25
- [ ] Lastfm scrobbling | 12/1
- [ ] Shuffle | 12/8
- [ ] Add rejoining if an unexpected disconnection happens | 12/15
Stretch goal
- [ ] Add member daemon type
  - [ ] -r flag with provided id joins existing lounge
  - [ ] -a auto joins the most recently created lounge
  - [ ] implement reaction based global pause and desync issues

## Dev Log

- 9/10 | Device ID generation is finished yay it was easy just hashing the machine ids so it stays consistant on a single device :P
- 9/11 | SocketIO connection is working!
- 9/13 | After figuring out how socketio kinda works I was able to grab the message for a user joining and the queue being updated
- 9/14 | I changed the workflow to using tokio MPSC which i barely understand so i need to learn more about that


<img src="https://nomnoml.com/image.svg?source=%5B%3Cstart%3Estart%5D%0A%0A%5Bstart%5D%20-%3E%0A%5B%20login%20%7C%0A%5Bgenerate%20device%20id%5D%20-%3E%0A%5Bpost%20%2Fv4%2Fcode%5D%20-%3E%0A%5Bpost%20%2Fv5%2Flogin%2Fcode%5D%20-%3E%0A%5Bcreate%20session%20from%20login%20info%5D%20-%3E%0A%5Bspit%20out%20refresh%20token%5D%0A%5D%0A%0A%5Bstart%5D%20-%3E%0A%5B%20Daemon%20%7C%0A%5B%20Queue%20Manager%20%7C%0A%5Bread%20AWAFY_TOKEN%20env%20var%5D%20-%3E%0A%5Bfetch%20queue%5D%20-%3E%0A%5Bconnect%20to%20websocket%20%7C%0A%5Bkeep%20track%20of%20lounge%20state%5D%0A%5Blisten%20for%20unpause%20and%20repause%5D%0A%5Bkeep%20lounge%20queue%20in%20sync%20with%20local%5D%0A%5D%0A%5D%0A%0A%5BPlayer%20%7C%0A%5Bread%20top%20song%20from%20queue%5D%20-%3E%0A%5Bstream%20song%20audio%20%26%20update%20plater%20state%20%7C%0A%5Brequest%20manifest%5D%20-%3E%0A%5Bstream%20audio%20from%20manifest%20data%5D%0A%5D%0A%5D%0A%5D%0A%0A%0A%5BModels%20%7C%0A%5Blounge%20%7C%20%0ALounge%20ID%0AName%0AState%0AQueue%0A%5D%0A%5BSession%20%7C%0AAccess%20token%0AToken%20expiry%0ARefresh%20token%0ADevice%20ID%0A%5D%0A%5BSong%20%7C%0AAlbum%20art%0ASong%20name%0AAlbum%0ATrack%20ID%0A%5D%0A%5BPlayerState%20%7C%0APlaying%0APosition%0ASong%0A%5D%0A%5BCode%20%7C%0ACode%0A%5D%0A%5D%0A%0A%0A%0A%0A%0A%0A%0A%0A%0A"/>
