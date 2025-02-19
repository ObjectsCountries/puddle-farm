use crate::{requests, responses, responses::Response};
use aes_gcm::{
    aead::{generic_array::GenericArray, Aead},
    Aes256Gcm, KeyInit,
};
use hex;
use lazy_static::lazy_static;
use reqwest::header;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use std::{error::Error, ops::Deref};
use tokio::sync::Mutex;

lazy_static! {
    pub static ref TOKEN: Mutex<Option<String>> = Mutex::new(None);
}

pub async fn get_player_stats(player_id: String) -> Result<String, String> {
    let request_data = requests::generate_player_stats_request(player_id);
    let request_data = encrypt_data(&request_data);

    let client = reqwest::Client::new();
    let form = client
        .post("https://ggst-game.guiltygear.com/api/statistics/get")
        .header(header::USER_AGENT, "GGST/Steam")
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("x-client-version", "1")
        .form(&[("data", request_data)]);

    let response = form.send().await.unwrap();
    let response_bytes = response.bytes().await.unwrap();

    if let Ok(r) = decrypt_response::<responses::PlayerStats>(&response_bytes) {
        Ok(r.body.json)
    } else {
        return Err("Couldn't get player stats".to_owned());
    }
}


pub async fn get_player_avatar(player_id: String) -> Result<String, String> {
  let request_data = requests::generate_player_avatar_request(player_id);
  let request_data = encrypt_data(&request_data);

  let client = reqwest::Client::new();
  let form = client
      .post("https://ggst-game.guiltygear.com/api/tus/read")
      .header(header::USER_AGENT, "GGST/Steam")
      .header(header::CACHE_CONTROL, "no-store")
      .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
      .header("x-client-version", "1")
      .form(&[("data", request_data)]);

  let response = form.send().await.unwrap();
  let response_bytes = response.bytes().await.unwrap();

  if let Ok(r) = decrypt_response::<responses::PlayerAvatar>(&response_bytes) {
      Ok(r.body.png)
  } else {
      return Err("Couldn't get player avatar".to_owned());
  }
}

pub async fn get_token() -> Result<String, String> {
    {
        let token = TOKEN.lock().await;
        if let Some(t) = token.deref() {
            debug!("Already have a strive token");
            return Ok(t.to_owned());
        }
    }

    warn!("Grabbing steam token");
    let request_data = requests::generate_login_request().await;
    let request_data = encrypt_data(&request_data);

    let client = reqwest::Client::new();
    let form = client
        .post("https://ggst-game.guiltygear.com/api/user/login")
        .header(header::USER_AGENT, "GGST/Steam")
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("x-client-version", "1")
        .form(&[("data", request_data)]);

    //Add logging before panic.
    let response = match form.send().await {
        Ok(resp) => resp,
        Err(err) => {
            error!("get_token send() error: {}", err);
            panic!(); //force crash to restart server
        }
    };

    let response_bytes = match response.bytes().await {
        Ok(resp_bytes) => resp_bytes,
        Err(err) => {
            error!("get_token bytes() error: {}", err);
            panic!(); //force crash to restart server
        }
    };
    
    info!("Waiting for strive token");

    let mut t = TOKEN.lock().await;

    if let Ok(r) = decrypt_response::<responses::Login>(&response_bytes) {
        info!("Got token: {}", r.header.token);
        *t = Some(r.header.token.to_owned());

        // save off token
        let _ = std::fs::write("token.txt", r.header.token.clone());

        Ok(r.header.token)
    } else {
        return Err("Couldn't get strive token".to_owned());
    }
}

pub async fn get_replays() -> Result<Vec<responses::Replay>, String> {
    let token = get_token().await?;
    let mut replays = Vec::new();
    for i in 0..5 {
        debug!("Grabbing replays (page {i})");
        let request_data = requests::generate_replay_request(i, 127, &token);
        let request_data = encrypt_data(&request_data);
        let client = reqwest::Client::new();
        let form = client
            .post("https://ggst-game.guiltygear.com/api/catalog/get_replay")
            .header(header::USER_AGENT, "GGST/Steam")
            .header(header::CACHE_CONTROL, "no-store")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header("x-client-version", "1")
            .form(&[("data", request_data)]);

        let response = match form.send().await {
            Ok(resp) => resp,
            Err(err) => {
                error!("get_replay send() error: {}", err);
                panic!("get_replay send() error: {}", err); //force crash to restart server
            }
        };

        let response_bytes = match response.bytes().await {
            Ok(resp_bytes) => resp_bytes,
            Err(err) => {
                error!("get_replay bytes() error: {}", err);
                panic!("get_replay bytes() error: {}", err); //force crash to restart server
            }
        };

        match decrypt_response::<responses::Replays>(&response_bytes) {
            Ok(r) => replays.extend_from_slice(&r.body.replays),
            Err(err) => {
                error!("get_replay decrypt_response() error: {}", err);
                panic!("get_replay decrypt_response() error: {}", err); //force crash to restart server
            }
        };
    }

    Ok(replays)
}

fn encrypt_data<T: Serialize>(data: &T) -> String {
    let key =
        hex::decode("EEBC1F57487F51921C0465665F8AE6D1658BB26DE6F8A069A3520293A572078F").unwrap();

    let bytes = rmp_serde::to_vec(data).unwrap();
    //let mut nonce = [0u8; 12];
    //getrandom(&mut nonce).unwrap();
    let nonce: [u8; 12] = *b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
    let nonce_ga = nonce.into();

    let aes_gcm = Aes256Gcm::new_from_slice(&key).unwrap();
    let encrypted = aes_gcm.encrypt(&nonce_ga, &bytes[..]).unwrap();

    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&nonce);
    data.extend_from_slice(&encrypted);

    let r = base64_url::encode(&data);

    r
}

fn decrypt_response<T: for<'a> Deserialize<'a>>(
    bytes: &[u8],
) -> Result<Response<T>, Box<dyn Error>> {
    let key =
        hex::decode("EEBC1F57487F51921C0465665F8AE6D1658BB26DE6F8A069A3520293A572078F").unwrap();
    let aes_gcm = Aes256Gcm::new_from_slice(&key).unwrap();

    let mut nonce = [0; 12];
    for i in 0..12 {
        nonce[i] = bytes[i];
    }

    //let nonce: GenericArray<_, _> = todo!();// GenericArray::from(&response_bytes[..12]);
    let nonce = GenericArray::from(nonce);

    let decrypted = match aes_gcm.decrypt(&nonce, &bytes[12..]) {
        Ok(decrypted) => decrypted,
        Err(e) => {
            panic!("Error decrypting: {:?}", e);
        }
    };

    match rmp_serde::from_slice::<responses::Response<T>>(&decrypted) {
        Ok(r) => Ok(r),
        Err(e) => {
            error!("Error in received msgpack!");
            let mut owned_string: String = "".to_owned();

            for b in &decrypted {
                owned_string.push_str(&format!("{:02X}", b));
            }
            error!("{:?}", owned_string);

            Err(Box::new(e))
        }
    }
}
