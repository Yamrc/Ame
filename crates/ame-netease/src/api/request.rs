use serde::de::DeserializeOwned;
use serde_json::Value;

pub trait ApiRequest {
    type Response: DeserializeOwned + Send + 'static;

    fn endpoint(&self) -> &'static str;
    fn payload(&self) -> Value;
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::ApiRequest;

    struct DummyRequest;

    impl ApiRequest for DummyRequest {
        type Response = Value;

        fn endpoint(&self) -> &'static str {
            "/dummy"
        }

        fn payload(&self) -> Value {
            json!({ "a": 1 })
        }
    }

    fn generic_accept<R: ApiRequest>(r: R) -> &'static str {
        r.endpoint()
    }

    #[test]
    fn trait_is_generic_ready() {
        let endpoint = generic_accept(DummyRequest);
        assert_eq!(endpoint, "/dummy");
    }
}
