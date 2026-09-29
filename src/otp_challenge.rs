use std::io::Write;

use hmac::{KeyInit, Mac};
use quinn::{RecvStream, SendStream};
use sha2::Sha256;
use wincode::{SchemaRead, SchemaWrite};

use crate::{AppResult, apperror::AppError::XCustomError, quic_client::Client};

type Hmac256 = hmac::Hmac<Sha256>;
#[derive(SchemaWrite, SchemaRead, Debug)]
pub enum HostMessage {
    OtpChallenge {
        nonce: [u8; 32],
    },
    SessionApproved {
        width: u32,
        height: u32,
        sample_rate: u32,
        channel: u8,
    },
    SessionDenied {
        reason: String,
    },
}

#[derive(SchemaWrite, SchemaRead, Debug)]
pub enum ClientMessage {
    OtpResponse { hmac: [u8; 32] },
}

pub struct OtpChallenge {
    otp: String,
    nonce: [u8; 32],
}

impl OtpChallenge {
    fn new(nonce: [u8; 32]) -> Self {
        Self {
            otp: String::new(),
            nonce,
        }
    }
    fn get_otp(&mut self, otp: String) {
        self.otp = otp
    }
    fn compute_hmac(&self) -> AppResult<[u8; 32]> {
        let mut mac = Hmac256::new_from_slice(self.otp.as_bytes())?;
        mac.update(&self.nonce);
        Ok(mac.finalize().into_bytes().into())
    }
}

pub async fn otp_challenge(
    recv: &mut RecvStream,
    send: &mut SendStream,
) -> AppResult<(u32, u32, u32, u8)> {
    let message = Client::read_message::<HostMessage>(recv).await?;
    if let HostMessage::OtpChallenge { nonce } = message {
        print!("Enter OTP: ");
        std::io::stdout().flush()?;
        let mut otp = String::new();
        std::io::stdin().read_line(&mut otp)?;
        let otp = otp.trim().to_string();
        let mut otp_generate = OtpChallenge::new(nonce);
        otp_generate.get_otp(otp);
        let hmac = otp_generate.compute_hmac()?;
        Client::send_message(send, &ClientMessage::OtpResponse { hmac }).await?;
    };
    let response = Client::read_message::<HostMessage>(recv).await?;

    match response {
        HostMessage::SessionApproved {
            width: w,
            height: h,
            sample_rate: s,
            channel: c,
        } => {
            println!("Session approved!");
            return Ok((w, h, s, c));
        }
        HostMessage::SessionDenied { reason } => {
            println!("Session denied!");
            let reason = Box::leak(reason.into_boxed_str());
            return Err(XCustomError(reason));
        }
        _ => {}
    }
    Err(XCustomError("Error in otp challenge"))
}
