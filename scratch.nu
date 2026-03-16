#!/usr/bin/env nu

const BASE_URL = "https://api.awa.io"
const LOGIN_FILE = $"($nu.home-dir)/.cache/AWAfy/login.json"

mkdir ($LOGIN_FILE | path dirname)


  let login_data = try { open $LOGIN_FILE } catch { {} }
  let username = $login_data.username? | default ""
  let user_id = $login_data.id? | default ""
  let token_expiry = ($login_data.token_expiry? | default "0" | into int)
  let device_id = $login_data.device_id? | default ""

  let refresh_token = try { secret-tool lookup service AWAfy username $username token_type refresh } catch { "" }
  let access_token = try { secret-tool lookup service AWAfy username $username token_type access } catch { "" }

  let is_logged_in = ($username != "") and ($device_id != "") and ($refresh_token != "")

  let login_status = {
    is_logged_in: $is_logged_in
    device_id: $device_id
    username: $username
    user_id: $user_id
    token_expiry: $token_expiry
    refresh_token: $refresh_token
    access_token: $access_token
  }
 
def main [] {}

def "main auth" [
  --qr-code (-q) # output login qr code along with text code
] {
  if $login_status.is_logged_in {
    let response = input $"User ($login_status.username) is already logged in. Do you want to overwrite? [y/N]: "
    if ($response | str downcase) != "y" {
      return
    }
  }

  let device_id = random uuid  

  let code = http post $"($BASE_URL)/v4/code" "" --content-type "application/json" --headers {
    X-Device-Id: $device_id
    X-Device-Id-Type: "4"
  }
  
  print $"Code: ($code.code)"
  if $qr_code {
    ^qrencode $"fmawa://settings/activation?code=($code.code)" -t ANSIUTF8 -m 2 -s 3
  }
  
  # Login with the activated code
  loop {
    let response = http post $"($BASE_URL)/v5/login/code" ($code | to json) -t "application/json" -fe --headers {
      X-Device-Id: $device_id
      X-Device-Id-Type: "4"
    }
    if ($response.status == 200) {
      print $"Authentication successful!\nLogged in as ($response.body.name)."
      $response.body.authData.refresh.refreshToken | secret-tool store --label='AWAfy Refresh Token' service AWAfy username $response.body.name token_type refresh
      $response.body.authData.refresh.accessToken | secret-tool store --label='AWAfy Access Token' service AWAfy username $response.body.name token_type access
      {
        username: $response.body.name
        user_id: $response.body.id
        token_expiry: $response.body.authData.refresh.expiresAt
        device_id: $device_id
      } | to json | save -f $LOGIN_FILE
      break
    }
    sleep 3sec
  }
}

def "main daemon" [] {
 
  if not $login_status.is_logged_in {
    error make {msg: "Not logged in. Please run `main auth` first."}
  }

  let device_id = $login_status.device_id
  let username = $login_status.username
  let username = $login_status.user_id
  mut token_expiry = $login_status.token_expiry
  let refresh_token = $login_status.refresh_token
  mut access_token = $login_status.access_token

  let current_time = (date now | format date "%s" | into int)
  
  if ($access_token == "") or (($current_time + 5) >= $token_expiry) {
    # Need to refresh token
    let response = http post $"($BASE_URL)/v5/authorize" ({refreshToken: $refresh_token} | to json) --content-type "application/json" --headers {
      X-Device-Id: $device_id
      X-Device-Id-Type: "4"
    }

    $token_expiry = ($response.authData.refresh.expiresAt | into int)
    $access_token = $response.authData.refresh.accessToken
    
    # Save the updated expiration time
    {username: $username, token_expiry: $token_expiry, device_id: $device_id} | to json | save -f $LOGIN_FILE
    
    # Store the new access token
    echo $access_token | secret-tool store --label='AWAfy Access Token' service AWAfy username $username token_type access
  }

  print "Starting daemon with active token..."
  http post $"($BASE_URL)/v6/room" ({name: "test" description: "" topicText: "" allowGifting: false allowLiveAudio: false disabledAutoFillTrack: false coOwnerUsers: [] thumbType: 2 backgroundType: 3} | to json) -fe -t "application/json" --headers {
    X-Device-Id: $device_id
    X-Device-Id-Type: "4"
    X-Access-Token: $access_token
  }
} 
