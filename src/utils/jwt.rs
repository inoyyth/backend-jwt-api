use chrono::{Duration, Utc};
use jsonwebtoken::{
    DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::Error as JwtError,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claims {
    pub sub: i64, // for user id
    pub exp: usize,
}

// function for generate token jwt
pub fn generate_token(user_id: i64) -> Result<String, JwtError> {
    // set expired in 24 hours
    let exp = Utc::now()
        .checked_add_signed(Duration::seconds(
            std::env::var("JWT_EXPIRATION")
                .unwrap_or_else(|_| "86400".to_string())
                .parse::<i64>()
                .unwrap(),
        ))
        .unwrap()
        .timestamp() as usize;

    // set token claims
    encode(
        &Header::default(),
        &Claims { sub: user_id, exp },
        &EncodingKey::from_secret(
            std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "secret".to_string())
                .as_ref(),
        ),
    )
}

//function for verify token jwt
pub fn verify_token(token: &str) -> Result<Claims, JwtError> {
    // Decode token then verify
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(
            std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "secret".to_string())
                .as_ref(),
        ),
        &Validation::default(),
    )?;

    // return token data
    Ok(token_data.claims)
}
