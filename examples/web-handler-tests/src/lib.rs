//! Testing request handlers with `test-better`.
//!
//! A request handler is a function from a [`Request`] to a [`Response`]. That
//! shape is a good fit for `test-better`: each test states a request, calls
//! [`route`], and makes a handful of `check!` assertions on the response. A
//! failure names the field that was wrong (`status`, `body`) instead of
//! dropping an `assert_eq!` panic on the harness.
//!
//! Run the suite with `cargo test -p web-handler-tests-example`.

/// An incoming request: just the method and path, enough to route on.
#[derive(Debug, Clone)]
pub struct Request {
    /// The HTTP method, uppercased (`"GET"`, `"POST"`, ...).
    pub method: String,
    /// The request path, including the leading slash.
    pub path: String,
}

impl Request {
    /// Builds a request from a method and path.
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
        }
    }
}

/// The handler's reply: a status code and a text body.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    /// The HTTP status code.
    pub status: u16,
    /// The response body.
    pub body: String,
}

impl Response {
    fn ok(body: &str) -> Self {
        Self {
            status: 200,
            body: body.to_string(),
        }
    }

    fn not_found() -> Self {
        Self {
            status: 404,
            body: "not found".to_string(),
        }
    }

    fn bad_request(why: &str) -> Self {
        Self {
            status: 400,
            body: why.to_string(),
        }
    }
}

/// Routes a request to a response.
///
/// The toy routing table:
/// - `GET /` greets.
/// - `GET /users/{id}` echoes a numeric user id; a non-numeric id is a 400.
/// - anything else is a 404.
pub fn route(request: &Request) -> Response {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => Response::ok("welcome"),
        ("GET", path) => match path.strip_prefix("/users/") {
            Some(id) if id.parse::<u64>().is_ok() => Response::ok(&format!("user {id}")),
            Some(_) => Response::bad_request("user id must be numeric"),
            None => Response::not_found(),
        },
        _ => Response::not_found(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_better::matches_struct;
    use test_better::prelude::*;

    #[test]
    fn the_index_route_greets() -> TestResult {
        let response = route(&Request::new("GET", "/"));
        check!(response.status).satisfies(eq(200))?;
        check!(response.body).satisfies(eq(String::from("welcome")))?;
        Ok(())
    }

    #[test]
    fn a_known_user_route_echoes_the_id() -> TestResult {
        let response = route(&Request::new("GET", "/users/42"));
        // `matches_struct!` checks several fields at once and, on a failure,
        // names the field that did not match.
        check!(response).satisfies(matches_struct!(Response {
            status: eq(200u16),
            body: contains_str("42"),
        }))?;
        Ok(())
    }

    #[test]
    fn a_non_numeric_user_id_is_a_bad_request() -> TestResult {
        let response = route(&Request::new("GET", "/users/not-a-number"));
        check!(response.status).satisfies(eq(400))?;
        check!(response.body).satisfies(contains_str("numeric"))?;
        Ok(())
    }

    #[test]
    fn an_unknown_path_is_a_not_found() -> TestResult {
        let response = route(&Request::new("GET", "/nope"));
        check!(response.status).satisfies(eq(404))?;
        Ok(())
    }

    #[test]
    fn a_wrong_method_is_a_not_found() -> TestResult {
        let response = route(&Request::new("POST", "/"));
        check!(response.status).satisfies(eq(404))?;
        Ok(())
    }

    #[test]
    fn one_run_can_check_every_route_with_soft() -> TestResult {
        // `soft` collects every failure in the closure instead of stopping at
        // the first, so a broken routing table shows all its breakage at once.
        soft(|s| {
            s.check(&route(&Request::new("GET", "/")).status, eq(200));
            s.check(&route(&Request::new("GET", "/users/7")).status, eq(200));
            s.check(&route(&Request::new("GET", "/missing")).status, eq(404));
        })
    }
}
