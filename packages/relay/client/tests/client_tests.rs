use chadreview_relay_client::RelayClient;
use chadreview_relay_models::PrKey;

#[test]
fn test_get_or_create_instance_id() {
    let id1 = RelayClient::get_or_create_instance_id();
    let id2 = RelayClient::get_or_create_instance_id();

    assert!(!id1.is_empty());
    assert!(!id2.is_empty());
    assert_eq!(id1, id2);
}

#[test]
fn test_instance_id_is_valid_uuid() {
    let id = RelayClient::get_or_create_instance_id();

    let parts: Vec<&str> = id.split('-').collect();
    assert_eq!(parts.len(), 5);

    assert_eq!(parts[0].len(), 8);
    assert_eq!(parts[1].len(), 4);
    assert_eq!(parts[2].len(), 4);
    assert_eq!(parts[3].len(), 4);
    assert_eq!(parts[4].len(), 12);
}

#[test]
fn test_pr_key_creation() {
    let pr_key = PrKey {
        owner: "test-owner".to_string(),
        repo: "test-repo".to_string(),
        number: 42,
    };

    assert_eq!(pr_key.owner, "test-owner");
    assert_eq!(pr_key.repo, "test-repo");
    assert_eq!(pr_key.number, 42);
}

#[test]
fn test_pr_key_equality() {
    let pr1 = PrKey {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        number: 1,
    };

    let pr2 = PrKey {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        number: 1,
    };

    assert_eq!(pr1, pr2);
}
