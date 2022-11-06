use std::sync::Arc;

use conduit::{Handler, HandlerResult, RequestExt};
use conduit_router::{RequestParams, RouteBuilder, RoutePattern};

use crate::controllers::*;
use crate::middleware::app::RequestApp;
use crate::middleware::log_request::add_custom_metadata;
use crate::util::errors::{std_error, AppError, RouteBlocked};
use crate::util::EndpointResult;
use crate::{App, Env};

pub fn build_router(app: &App) -> RouteBuilder {
    let mut router = RouteBuilder::new();

    // Route used by both `cargo search` and the frontend
    router.get("/api/v1/crates", C(krate::search::search));

    // Routes used by `cargo`
    router.put("/api/v1/crates/new", C(krate::publish::publish));
    router.get("/api/v1/crates/:crate_id/owners", C(krate::owners::owners));
    router.put(
        "/api/v1/crates/:crate_id/owners",
        C(krate::owners::add_owners),
    );
    router.delete(
        "/api/v1/crates/:crate_id/owners",
        C(krate::owners::remove_owners),
    );
    router.delete(
        "/api/v1/crates/:crate_id/:version/yank",
        C(version::yank::yank),
    );
    router.put(
        "/api/v1/crates/:crate_id/:version/unyank",
        C(version::yank::unyank),
    );
    router.get(
        "/api/v1/crates/:crate_id/:version/download",
        C(version::downloads::download),
    );

    // Routes that appear to be unused
    router.get("/api/v1/versions", C(version::deprecated::index));
    router.get(
        "/api/v1/versions/:version_id",
        C(version::deprecated::show_by_id),
    );

    // Routes used by the frontend
    router.get("/api/v1/crates/:crate_id", C(krate::metadata::show));
    router.get(
        "/api/v1/crates/:crate_id/:version",
        C(version::metadata::show),
    );
    router.get(
        "/api/v1/crates/:crate_id/:version/readme",
        C(krate::metadata::readme),
    );
    router.get(
        "/api/v1/crates/:crate_id/:version/dependencies",
        C(version::metadata::dependencies),
    );
    router.get(
        "/api/v1/crates/:crate_id/:version/downloads",
        C(version::downloads::downloads),
    );
    router.get(
        "/api/v1/crates/:crate_id/:version/authors",
        C(version::metadata::authors),
    );
    router.get(
        "/api/v1/crates/:crate_id/downloads",
        C(krate::downloads::downloads),
    );
    router.get(
        "/api/v1/crates/:crate_id/versions",
        C(krate::metadata::versions),
    );
    router.put("/api/v1/crates/:crate_id/follow", C(krate::follow::follow));
    router.delete(
        "/api/v1/crates/:crate_id/follow",
        C(krate::follow::unfollow),
    );
    router.get(
        "/api/v1/crates/:crate_id/following",
        C(krate::follow::following),
    );
    router.get(
        "/api/v1/crates/:crate_id/owner_team",
        C(krate::owners::owner_team),
    );
    router.get(
        "/api/v1/crates/:crate_id/owner_user",
        C(krate::owners::owner_user),
    );
    router.get(
        "/api/v1/crates/:crate_id/reverse_dependencies",
        C(krate::metadata::reverse_dependencies),
    );
    router.get("/api/v1/keywords", C(keyword::index));
    router.get("/api/v1/keywords/:keyword_id", C(keyword::show));
    router.get("/api/v1/categories", C(category::index));
    router.get("/api/v1/categories/:category_id", C(category::show));
    router.get("/api/v1/category_slugs", C(category::slugs));
    router.get("/api/v1/users/:user_id", C(user::other::show));
    router.put("/api/v1/users/:user_id", C(user::me::update_user));
    router.get("/api/v1/users/:user_id/stats", C(user::other::stats));
    router.get("/api/v1/teams/:team_id", C(team::show_team));
    router.get("/api/v1/me", C(user::me::me));
    router.get("/api/v1/me/updates", C(user::me::updates));
    router.get("/api/v1/me/tokens", C(token::list));
    router.put("/api/v1/me/tokens", C(token::new));
    router.delete("/api/v1/me/tokens/:id", C(token::revoke));
    router.delete("/api/v1/tokens/current", C(token::revoke_current));
    router.get(
        "/api/v1/me/crate_owner_invitations",
        C(crate_owner_invitation::list),
    );
    router.put(
        "/api/v1/me/crate_owner_invitations/:crate_id",
        C(crate_owner_invitation::handle_invite),
    );
    router.put(
        "/api/v1/me/crate_owner_invitations/accept/:token",
        C(crate_owner_invitation::handle_invite_with_token),
    );
    router.put(
        "/api/v1/me/email_notifications",
        C(user::me::update_email_notifications),
    );
    router.get("/api/v1/summary", C(krate::metadata::summary));
    router.put(
        "/api/v1/confirm/:email_token",
        C(user::me::confirm_user_email),
    );
    router.put(
        "/api/v1/users/:user_id/resend",
        C(user::me::regenerate_token_and_send),
    );
    router.get("/api/v1/site_metadata", C(site_metadata::show_deployed_sha));

    // Session management
    router.get("/api/private/session/begin", C(user::session::begin));
    router.get(
        "/api/private/session/authorize",
        C(user::session::authorize),
    );
    router.delete("/api/private/session", C(user::session::logout));

    // Metrics
    router.get("/api/private/metrics/:kind", C(metrics::prometheus));

    // Crate ownership invitations management in the frontend
    router.get(
        "/api/private/crate_owner_invitations",
        C(crate_owner_invitation::private_list),
    );

    // Only serve the local checkout of the git index in development mode.
    // In production, for crates.io, cargo gets the index from
    // https://github.com/rust-lang/crates.io-index directly.
    if app.config.env() == Env::Development {
        let s = conduit_git_http_backend::Serve("./tmp/index-bare".into());
        let s = Arc::new(s);
        router.get("/git/index/*path", R(Arc::clone(&s)));
        router.post("/git/index/*path", R(s));
    }

    router.get("/", conduit_static::Static::new("dist"));

    router
}

struct C(pub fn(&mut dyn RequestExt) -> EndpointResult);

impl Handler for C {
    fn call(&self, req: &mut dyn RequestExt) -> HandlerResult {
        if let Some(pattern) = req.extensions().get::<RoutePattern>() {
            let pattern = pattern.pattern();

            // Configure the Sentry `transaction` field *before* we handle the request,
            // but *after* the conduit-router has figured out which handler to use.
            sentry::configure_scope(|scope| scope.set_transaction(Some(pattern)));

            // Allow blocking individual routes by their pattern through the `BLOCKED_ROUTES`
            // environment variable. This is not in a middleware because we need access to
            // `RoutePattern` before executing the response handler.
            if req.app().config.blocked_routes.contains(pattern) {
                return Ok(RouteBlocked.response().unwrap());
            }
        }

        let C(f) = *self;
        match f(req) {
            Ok(resp) => Ok(resp),
            Err(e) => {
                if let Some(cause) = e.cause() {
                    add_custom_metadata("cause", cause.to_string())
                };
                match e.response() {
                    Some(response) => Ok(response),
                    None => Err(std_error(e)),
                }
            }
        }
    }
}

struct R<H>(pub Arc<H>);

impl<H: Handler> Handler for R<H> {
    fn call(&self, req: &mut dyn RequestExt) -> HandlerResult {
        *req.path_mut() = req.params()["path"].to_string();
        let R(ref sub_router) = *self;
        sub_router.call(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::errors::{bad_request, cargo_err, forbidden, internal, not_found, AppError};
    use crate::util::EndpointResult;

    use conduit::StatusCode;
    use conduit_test::MockRequest;
    use diesel::result::Error as DieselError;

    fn err<E: AppError>(err: E) -> EndpointResult {
        Err(Box::new(err))
    }

    #[test]
    fn http_error_responses() {
        let mut req = MockRequest::new(::conduit::Method::GET, "/");

        // Types for handling common error status codes
        assert_eq!(
            C(|_| Err(bad_request(""))).call(&mut req).unwrap().status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            C(|_| Err(forbidden())).call(&mut req).unwrap().status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            C(|_| Err(DieselError::NotFound.into()))
                .call(&mut req)
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            C(|_| Err(not_found())).call(&mut req).unwrap().status(),
            StatusCode::NOT_FOUND
        );

        // cargo_err errors are returned as 200 so that cargo displays this nicely on the command line
        assert_eq!(
            C(|_| Err(cargo_err(""))).call(&mut req).unwrap().status(),
            StatusCode::OK
        );

        // Inner errors are captured for logging when wrapped by a user facing error
        let response = C(|_| {
            Err("-1"
                .parse::<u8>()
                .map_err(|err| err.chain(internal("middle error")))
                .map_err(|err| err.chain(bad_request("outer user facing error")))
                .unwrap_err())
        })
        .call(&mut req)
        .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            crate::middleware::log_request::get_log_message("cause"),
            "middle error caused by invalid digit found in string"
        );

        // All other error types are propogated up the middleware, eventually becoming status 500
        assert!(C(|_| Err(internal(""))).call(&mut req).is_err());
        assert!(
            C(|_| err::<::serde_json::Error>(::serde::de::Error::custom("ExpectedColon")))
                .call(&mut req)
                .is_err()
        );
        assert!(
            C(|_| err(::std::io::Error::new(::std::io::ErrorKind::Other, "")))
                .call(&mut req)
                .is_err()
        );
    }
}
