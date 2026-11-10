use super::*;

#[test]
fn the_params_never_print_a_token() {
    let params: SpareWarmParams = serde_json::from_value(serde_json::json!({
        "platform": "https://platform.example", "api": "https://api.example",
        "access_token": "stack.secret", "install_token": "install.secret", "account": "user_a",
        "claimed": "vm_0123456789abcdefghij", "login_scopes": ["team_a", null], "dogfood": true,
    }))
    .unwrap();
    let printed = format!("{params:?}");
    assert!(!printed.contains("secret"), "{printed}");
    assert_eq!(params.login_scopes, vec![Some("team_a".to_owned()), None]);
}

#[test]
fn an_unknown_field_is_refused() {
    let parsed = serde_json::from_value::<SpareWarmParams>(serde_json::json!({
        "platform": "p", "api": "a", "access_token": "t", "install_token": "i", "account": "user_a",
        "credential": "elsewhere",
    }));
    assert!(parsed.is_err());
}
