use uuid::Uuid;

pub fn random_user_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn internal_secret() -> &'static str {
    "test-internal-secret"
}

pub fn api_key() -> &'static str {
    "pk_test_1234567890"
}
