use super::*;

#[test]
fn retryable_status_matches_new_api_policy() {
    assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
    assert!(is_retryable_status(StatusCode::TEMPORARY_REDIRECT));
    assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));

    // new-api excludes 504/524 timeouts from retries.
    assert!(!is_retryable_status(StatusCode::GATEWAY_TIMEOUT));
    assert!(!is_retryable_status(StatusCode::from_u16(524).expect("524")));

    assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
    assert!(!is_retryable_status(StatusCode::UNAUTHORIZED));
}

