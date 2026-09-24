use crate::http::{Method, Request, Response, StatusCode};

type Handler = Box<dyn Fn(&Request) -> Response + Send + Sync>;

#[derive(Default)]
pub struct Router {
    routes: Vec<(Method, String, Handler)>,
}

impl Router {
    pub fn new() -> Router {
        Router::default()
    }

    pub fn route<H>(mut self, method: Method, path: &str, handler: H) -> Router
    where
        H: Fn(&Request) -> Response + Send + Sync + 'static,
    {
        self.routes
            .push((method, path.to_string(), Box::new(handler)));
        self
    }

    pub fn handle(&self, request: &Request) -> Response {
        let path = request.path();
        let find = |method: Method| {
            self.routes
                .iter()
                .find(|(m, p, _)| *m == method && p == path)
                .map(|(_, _, handler)| handler)
        };

        let handler = find(request.method()).or_else(|| match request.method() {
            Method::Head => find(Method::Get),
            _ => None,
        });
        if let Some(handler) = handler {
            return handler(request);
        }

        let mut allowed: Vec<&str> = Vec::new();
        for (method, _, _) in self.routes.iter().filter(|(_, p, _)| p == path) {
            allowed.push(method.as_str());
            if *method == Method::Get {
                allowed.push(Method::Head.as_str());
            }
        }
        if allowed.is_empty() {
            return Response::text(StatusCode::NOT_FOUND, "404 Not Found\n");
        }
        allowed.sort_unstable();
        allowed.dedup();
        Response::text(StatusCode::METHOD_NOT_ALLOWED, "405 Method Not Allowed\n")
            .header("Allow", &allowed.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::ServerLimits;
    use std::io::Cursor;

    fn request(head: &str) -> Request {
        let wire = format!("{head} HTTP/1.1\r\nHost: x\r\n\r\n");
        Request::read(&mut Cursor::new(wire.as_bytes()), &ServerLimits::default()).unwrap()
    }

    fn router() -> Router {
        Router::new()
            .route(Method::Get, "/", |_| Response::text(StatusCode::OK, "root"))
            .route(Method::Post, "/items", |r| {
                Response::text(StatusCode::CREATED, r.target())
            })
    }

    #[test]
    fn dispatches_on_method_and_path() {
        let response = router().handle(&request("GET /"));
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.body_bytes(), b"root");
    }

    #[test]
    fn query_string_is_not_part_of_the_match() {
        let response = router().handle(&request("POST /items?id=7"));
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.body_bytes(), b"/items?id=7");
    }

    #[test]
    fn unknown_path_is_404() {
        assert_eq!(
            router().handle(&request("GET /nope")).status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn known_path_wrong_method_is_405_with_allow() {
        let response = router().handle(&request("DELETE /"));
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        let mut wire = Vec::new();
        response.write_to(&mut wire, false, false).unwrap();
        assert!(
            String::from_utf8(wire)
                .unwrap()
                .contains("Allow: GET, HEAD\r\n")
        );
    }

    #[test]
    fn head_falls_back_to_the_get_handler() {
        assert_eq!(router().handle(&request("HEAD /")).status(), StatusCode::OK);
        assert_eq!(
            router().handle(&request("HEAD /items")).status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
    }
}
