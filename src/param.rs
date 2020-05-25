//! Extension method for hyper requests for accessing query parameters
use std::collections::HashMap;

pub(crate) trait Params {
    fn query_params(&self) -> HashMap<String, String>;
    fn query_param(
        &self,
        name: &str,
    ) -> Option<String> {
        self.query_params().get(name).map(String::clone)
    }
}

impl<T> Params for hyper::Request<T> {
    fn query_params(&self) -> HashMap<String, String> {
        self.uri()
            .query()
            .map(|v| {
                url::form_urlencoded::parse(v.as_bytes())
                    .into_owned()
                    .collect()
            })
            .unwrap_or_else(HashMap::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_query_params() -> Result<(), Box<dyn std::error::Error>> {
        let mut expect = HashMap::new();
        expect.insert("baz".to_string(), "boom".to_string());
        assert_eq!(
            hyper::Request::builder()
                .uri("https://foo.bar?baz=boom")
                .body(())?
                .query_params(),
            expect
        );
        Ok(())
    }

    #[test]
    fn request_query_param() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            hyper::Request::builder()
                .uri("https://foo.bar?baz=boom")
                .body(())?
                .query_param("baz"),
            Some("boom".into())
        );
        Ok(())
    }
}
