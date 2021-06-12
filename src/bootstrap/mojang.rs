use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha1::Sha1;
use crate::error::{Res, MojangErr};
use packets::types::UUID;
use reqwest::StatusCode;

pub struct Mojang {
    client: reqwest::Client,
}

impl Default for Mojang {
    fn default() -> Self {
        Mojang {
            client: reqwest::Client::new()
        }
    }
}

pub fn calc_hash(server_id: &str, shared_secret: &[u8], public_key_encoded: &[u8]) -> String {
    let ascii = server_id.as_bytes();
    let mut sha1 = Sha1::new();
    sha1.update(ascii);
    sha1.update(shared_secret);
    sha1.update(public_key_encoded);
    hexdigest(&sha1.digest().bytes())
}

fn hexdigest(bytes: &[u8]) -> String {
    let bigint = BigInt::from_signed_bytes_be(&bytes);
    format!("{:x}", bigint)
}



#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SelectedProfile {
    pub name: String,
    pub id: String
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub access_token: String,
    pub selected_profile: SelectedProfile,
}

impl Mojang {
    pub async fn authenticate(&self, email: &str, password: &str) -> Res<AuthResponse> {
        let payload = json!({
            "agent": {
                "name": "Minecraft",
                "version": 1
            },
            "username": email, // this is not a mistake... the username now takes in email
            "password": password,
            "requestUser": false
        });

        let payload = payload.to_string();

        // TODO: is this right?
        let res = self.client.post("https://authserver.mojang.com/authenticate")
            .body(payload)
            .send()
            .await?;


        let status = res.status();
        if status != 200 {
            return Err(MojangErr::InvalidCredentials {
                error_code: status,
                info: res.text().await.ok()
            }.into());
        }

        let auth: AuthResponse  = res.json().await?;
        Ok(auth)
    }

    pub async fn join(&self, uuid: UUID, server_hash: &str, access_token: &str) -> Res<()> {
        let uuid_str = uuid.to_string();

        let payload = json!({
            "accessToken": access_token,
            "selectedProfile": uuid_str,
            "serverId": server_hash
        });

        let payload = payload.to_string();

        let res = self.client.post("https://sessionserver.mojang.com/session/minecraft/join")
            .body(payload)
            .send()
            .await?;

        let status = res.status();
        if status != 204 {

            println!("uuid invalid {}", uuid_str);
            return Err(MojangErr::InvalidCredentials {
                error_code: status,
                info: res.text().await.ok()
            }.into());
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use sha1::Sha1;

    use crate::mojang::hexdigest;
    use crate::bootstrap::mojang::hexdigest;

    fn sha1(input: &[u8]) -> String {
        let mut sha1 = Sha1::new();
        sha1.update(input);
        hexdigest(&sha1.digest().bytes())
    }

    #[test]
    fn test_hash() {
        assert_eq!(sha1(b"jeb_"), "-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1");
        assert_eq!(sha1(b"simon"), "88e16a1019277b15d58faf0541e11910eb756f6");
        assert_eq!(sha1(b"Notch"), "4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48");
    }
}
