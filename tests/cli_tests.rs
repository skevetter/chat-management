use assert_cmd::Command;
use predicates::prelude::*;
use rusqlite::params;
use tempfile::TempDir;

fn cmd(tmp: &TempDir) -> Command {
    let db_path = tmp.path().join("test.db");
    let mut cmd = Command::cargo_bin("chat-management").unwrap();
    cmd.arg("--db").arg(db_path.to_str().unwrap());
    cmd
}

fn cmd_ns(tmp: &TempDir, ns: &str) -> Command {
    let db_path = tmp.path().join("test.db");
    let mut cmd = Command::cargo_bin("chat-management").unwrap();
    cmd.arg("--db")
        .arg(db_path.to_str().unwrap())
        .arg("--namespace")
        .arg(ns);
    cmd
}

fn cmd_json(tmp: &TempDir) -> Command {
    let db_path = tmp.path().join("test.db");
    let mut cmd = Command::cargo_bin("chat-management").unwrap();
    cmd.arg("--db")
        .arg(db_path.to_str().unwrap())
        .arg("--output")
        .arg("json");
    cmd
}

fn cmd_csv(tmp: &TempDir) -> Command {
    let db_path = tmp.path().join("test.db");
    let mut cmd = Command::cargo_bin("chat-management").unwrap();
    cmd.arg("--db")
        .arg(db_path.to_str().unwrap())
        .arg("--output")
        .arg("csv");
    cmd
}

// === Channel CRUD ===

#[test]
fn test_channel_create() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "channel",
            "create",
            "--name",
            "general",
            "--purpose",
            "General chat",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("general"))
        .stdout(predicate::str::contains("General chat"));
}

#[test]
fn test_channel_create_json() {
    let tmp = TempDir::new().unwrap();
    let output = cmd_json(&tmp)
        .args([
            "channel",
            "create",
            "--name",
            "dev",
            "--purpose",
            "Dev channel",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["name"], "dev");
    assert_eq!(parsed["purpose"], "Dev channel");
    assert_eq!(parsed["namespace"], "default");
    assert_eq!(parsed["message_count"], 0);
}

#[test]
fn test_channel_list() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "ch1"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "create", "--name", "ch2"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ch1"))
        .stdout(predicate::str::contains("ch2"));
}

#[test]
fn test_channel_list_json() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "alpha"])
        .assert()
        .success();
    let output = cmd_json(&tmp).args(["channel", "list"]).output().unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 1);
    assert_eq!(parsed["channels"][0]["name"], "alpha");
}

#[test]
fn test_channel_show() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "mychannel"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "show", "mychannel"])
        .assert()
        .success()
        .stdout(predicate::str::contains("mychannel"));
}

#[test]
fn test_channel_show_by_id() {
    let tmp = TempDir::new().unwrap();
    let output = cmd_json(&tmp)
        .args(["channel", "create", "--name", "idtest"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let id = parsed["id"].as_i64().unwrap().to_string();
    cmd(&tmp)
        .args(["channel", "show", &id])
        .assert()
        .success()
        .stdout(predicate::str::contains("idtest"));
}

#[test]
fn test_channel_delete() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "todelete"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "delete", "todelete"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted"));
    cmd(&tmp)
        .args(["channel", "show", "todelete"])
        .assert()
        .failure();
}

// === Message Posting ===

#[test]
fn test_post_message_basic() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "msgs"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "msgs",
            "--sender",
            "agent-1",
            "--content",
            "Hello world",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello world"))
        .stdout(predicate::str::contains("agent-1"));
}

#[test]
fn test_post_message_json() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "jsonch"])
        .assert()
        .success();
    let output = cmd_json(&tmp)
        .args(["post", "jsonch", "--sender", "bot", "--content", "Test msg"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["sender"], "bot");
    assert_eq!(parsed["content"], "Test msg");
    assert!(!parsed["id"].as_str().unwrap().is_empty());
}

#[test]
fn test_post_message_with_reply_to() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "replies"])
        .assert()
        .success();
    let output = cmd_json(&tmp)
        .args(["post", "replies", "--sender", "a", "--content", "first"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let msg_id = parsed["id"].as_str().unwrap().to_string();

    let output2 = cmd_json(&tmp)
        .args([
            "post",
            "replies",
            "--sender",
            "b",
            "--content",
            "reply",
            "--reply-to",
            &msg_id,
        ])
        .output()
        .unwrap();
    assert!(output2.status.success());
    let parsed2: serde_json::Value = serde_json::from_slice(&output2.stdout).unwrap();
    assert_eq!(parsed2["reply_to"], msg_id);
}

#[test]
fn test_post_message_with_idempotency_key() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "idemp"])
        .assert()
        .success();
    let output = cmd_json(&tmp)
        .args([
            "post",
            "idemp",
            "--sender",
            "x",
            "--content",
            "unique",
            "--idempotency-key",
            "key-123",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["idempotency_key"], "key-123");
}

// === Idempotency Dedup ===

#[test]
fn test_idempotency_dedup_same_message() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "dedup"])
        .assert()
        .success();

    let output1 = cmd_json(&tmp)
        .args([
            "post",
            "dedup",
            "--sender",
            "agent",
            "--content",
            "msg1",
            "--idempotency-key",
            "dedup-key",
        ])
        .output()
        .unwrap();
    let parsed1: serde_json::Value = serde_json::from_slice(&output1.stdout).unwrap();
    let id1 = parsed1["id"].as_str().unwrap().to_string();

    let output2 = cmd_json(&tmp)
        .args([
            "post",
            "dedup",
            "--sender",
            "agent",
            "--content",
            "msg1",
            "--idempotency-key",
            "dedup-key",
        ])
        .output()
        .unwrap();
    assert!(output2.status.success());
    let parsed2: serde_json::Value = serde_json::from_slice(&output2.stdout).unwrap();
    let id2 = parsed2["id"].as_str().unwrap().to_string();

    assert_eq!(id1, id2);
}

#[test]
fn test_idempotency_different_keys_create_different() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "dedup2"])
        .assert()
        .success();

    let output1 = cmd_json(&tmp)
        .args([
            "post",
            "dedup2",
            "--sender",
            "a",
            "--content",
            "x",
            "--idempotency-key",
            "key-a",
        ])
        .output()
        .unwrap();
    let parsed1: serde_json::Value = serde_json::from_slice(&output1.stdout).unwrap();

    let output2 = cmd_json(&tmp)
        .args([
            "post",
            "dedup2",
            "--sender",
            "a",
            "--content",
            "x",
            "--idempotency-key",
            "key-b",
        ])
        .output()
        .unwrap();
    let parsed2: serde_json::Value = serde_json::from_slice(&output2.stdout).unwrap();

    assert_ne!(
        parsed1["id"].as_str().unwrap(),
        parsed2["id"].as_str().unwrap()
    );
}

// === Message Reading ===

#[test]
fn test_read_messages_basic() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "readch"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "readch", "--sender", "a", "--content", "msg1"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "readch", "--sender", "b", "--content", "msg2"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["read", "readch"])
        .assert()
        .success()
        .stdout(predicate::str::contains("msg1"))
        .stdout(predicate::str::contains("msg2"));
}

#[test]
fn test_read_messages_with_limit() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "limitch"])
        .assert()
        .success();
    for i in 0..5 {
        cmd(&tmp)
            .args([
                "post",
                "limitch",
                "--sender",
                "x",
                "--content",
                &format!("msg{i}"),
            ])
            .assert()
            .success();
    }
    let output = cmd_json(&tmp)
        .args(["read", "limitch", "--limit", "2"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["messages"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["total"], 5);
}

#[test]
fn test_read_messages_with_since() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "sincech"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "sincech", "--sender", "a", "--content", "old msg"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["read", "sincech", "--since", "2099-01-01T00:00:00Z"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["messages"].as_array().unwrap().len(), 0);

    let output2 = cmd_json(&tmp)
        .args(["read", "sincech", "--since", "2000-01-01T00:00:00Z"])
        .output()
        .unwrap();
    let parsed2: serde_json::Value = serde_json::from_slice(&output2.stdout).unwrap();
    assert_eq!(parsed2["messages"].as_array().unwrap().len(), 1);
}

#[test]
fn test_read_messages_with_sender_filter() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "senderch"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "senderch",
            "--sender",
            "alice",
            "--content",
            "from alice",
        ])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "senderch",
            "--sender",
            "bob",
            "--content",
            "from bob",
        ])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["read", "senderch", "--sender", "alice"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["messages"].as_array().unwrap().len(), 1);
    assert_eq!(parsed["messages"][0]["sender"], "alice");
}

// === Mention Extraction ===

#[test]
fn test_mention_single() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "mentch"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "mentch",
            "--sender",
            "x",
            "--content",
            "Hey @agent-1 check this",
        ])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["mentions", "--agent", "agent-1"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 1);
    assert_eq!(parsed["mentions"][0]["mentioned_agent"], "agent-1");
}

#[test]
fn test_mention_multiple() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "multiment"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "multiment",
            "--sender",
            "x",
            "--content",
            "@alice @bob please review",
        ])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["mentions", "--channel", "multiment"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 2);
}

#[test]
fn test_mention_none() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "noment"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "noment",
            "--sender",
            "x",
            "--content",
            "no mentions here",
        ])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["mentions", "--channel", "noment"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 0);
}

#[test]
fn test_mention_dedup_within_message() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "dupment"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "dupment",
            "--sender",
            "x",
            "--content",
            "@alice @alice @alice",
        ])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["mentions", "--channel", "dupment"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 1);
}

// === Namespace Scoping ===

#[test]
fn test_namespace_create_and_list() {
    let tmp = TempDir::new().unwrap();
    cmd_ns(&tmp, "team-a")
        .args(["channel", "create", "--name", "shared"])
        .assert()
        .success();
    cmd_ns(&tmp, "team-b")
        .args(["channel", "create", "--name", "shared"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["--namespace", "team-a", "channel", "list"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 1);
    assert_eq!(parsed["channels"][0]["namespace"], "team-a");
}

#[test]
fn test_namespace_isolation() {
    let tmp = TempDir::new().unwrap();
    cmd_ns(&tmp, "ns1")
        .args(["channel", "create", "--name", "private"])
        .assert()
        .success();

    cmd_ns(&tmp, "ns2")
        .args(["channel", "show", "private"])
        .assert()
        .failure();
}

#[test]
fn test_namespace_messages_scoped() {
    let tmp = TempDir::new().unwrap();
    cmd_ns(&tmp, "ns-msg")
        .args(["channel", "create", "--name", "chat"])
        .assert()
        .success();
    cmd_ns(&tmp, "ns-msg")
        .args(["post", "chat", "--sender", "a", "--content", "hello"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["--namespace", "ns-msg", "read", "chat"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 1);
}

// === Inspect ===

#[test]
fn test_inspect_channel() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "channel",
            "create",
            "--name",
            "inspectable",
            "--purpose",
            "Inspection target",
        ])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "inspectable", "--sender", "x", "--content", "msg"])
        .assert()
        .success();

    cmd(&tmp)
        .args(["inspect", "inspectable"])
        .assert()
        .success()
        .stdout(predicate::str::contains("inspectable"))
        .stdout(predicate::str::contains("Inspection target"));
}

#[test]
fn test_inspect_channel_json() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "insjson"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "insjson", "--sender", "a", "--content", "x"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["inspect", "insjson"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["name"], "insjson");
    assert_eq!(parsed["message_count"], 1);
}

// === Edge Cases ===

#[test]
fn test_delete_nonexistent_channel() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "delete", "ghost"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_post_to_nonexistent_channel() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["post", "ghost", "--sender", "x", "--content", "hello"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_message_count_trigger() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "countch"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "countch", "--sender", "a", "--content", "1"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "countch", "--sender", "a", "--content", "2"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "countch", "--sender", "a", "--content", "3"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["inspect", "countch"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["message_count"], 3);
}

#[test]
fn test_channel_create_duplicate_fails() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "dup"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "create", "--name", "dup"])
        .assert()
        .failure();
}

// === Bug Fix: --since relative duration parsing ===

#[test]
fn test_read_since_relative_duration() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "relch"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "relch", "--sender", "a", "--content", "recent msg"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["read", "relch", "--since", "5m"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["messages"].as_array().unwrap().len(), 1);
}

#[test]
fn test_read_since_relative_hours() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "hrch"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "hrch", "--sender", "a", "--content", "msg"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["read", "hrch", "--since", "1h"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["messages"].as_array().unwrap().len(), 1);
}

#[test]
fn test_read_since_relative_seconds() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "secch"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "secch", "--sender", "a", "--content", "msg"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["read", "secch", "--since", "30s"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["messages"].as_array().unwrap().len(), 1);
}

#[test]
fn test_read_since_invalid_format_errors() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "badch"])
        .assert()
        .success();

    cmd(&tmp)
        .args(["read", "badch", "--since", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid since value"));
}

#[test]
fn test_read_since_iso_timestamp_still_works() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "isoch"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "isoch", "--sender", "a", "--content", "msg"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["read", "isoch", "--since", "2000-01-01T00:00:00Z"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["messages"].as_array().unwrap().len(), 1);
}

// === Bug Fix: JSON error formatting ===

#[test]
fn test_json_error_format_channel_not_found() {
    let tmp = TempDir::new().unwrap();
    let output = cmd_json(&tmp)
        .args(["channel", "show", "nonexistent"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert!(parsed["error"].as_str().unwrap().contains("not found"));
}

#[test]
fn test_json_error_format_post_to_nonexistent() {
    let tmp = TempDir::new().unwrap();
    let output = cmd_json(&tmp)
        .args(["post", "ghost", "--sender", "x", "--content", "hello"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert!(parsed["error"].as_str().unwrap().contains("not found"));
}

#[test]
fn test_json_error_format_invalid_since() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "errch"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["read", "errch", "--since", "garbage"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert!(
        parsed["error"]
            .as_str()
            .unwrap()
            .contains("Invalid since value")
    );
}

// === Bug Fix: Empty content validation ===

#[test]
fn test_post_empty_content_rejected() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "emptych"])
        .assert()
        .success();

    cmd(&tmp)
        .args(["post", "emptych", "--sender", "a", "--content", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be empty"));
}

#[test]
fn test_post_whitespace_only_content_rejected() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "wsch"])
        .assert()
        .success();

    cmd(&tmp)
        .args(["post", "wsch", "--sender", "a", "--content", "   "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be empty"));
}

#[test]
fn test_post_empty_content_json_error() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "emptyjch"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["post", "emptyjch", "--sender", "a", "--content", ""])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert!(
        parsed["error"]
            .as_str()
            .unwrap()
            .contains("cannot be empty")
    );
}

// === Feature: Message count in channel list ===

#[test]
fn test_channel_list_includes_message_count() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "cntch"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "cntch", "--sender", "a", "--content", "one"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "cntch", "--sender", "a", "--content", "two"])
        .assert()
        .success();

    let output = cmd_json(&tmp).args(["channel", "list"]).output().unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["channels"][0]["message_count"], 2);
}

#[test]
fn test_channel_list_table_shows_message_count() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "tblch"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["post", "tblch", "--sender", "a", "--content", "hi"])
        .assert()
        .success();

    cmd(&tmp)
        .args(["channel", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MSGS"))
        .stdout(predicate::str::contains("1"));
}

// === Search (FTS5) ===

#[test]
fn test_search_basic_keyword() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "search-ch"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "search-ch",
            "--sender",
            "alice",
            "--content",
            "the deploy failed on staging",
        ])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "search-ch",
            "--sender",
            "bob",
            "--content",
            "all tests passing now",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["search", "--query", "deploy"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deploy failed on staging"))
        .stdout(predicate::str::contains("alice"));
}

#[test]
fn test_search_channel_filter() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "ch-a"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "create", "--name", "ch-b"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "ch-a",
            "--sender",
            "alice",
            "--content",
            "hello world from channel a",
        ])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "ch-b",
            "--sender",
            "bob",
            "--content",
            "hello world from channel b",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["search", "--query", "hello", "--channel", "ch-a"])
        .assert()
        .success()
        .stdout(predicate::str::contains("channel a"))
        .stdout(predicate::str::contains("channel b").not());
}

#[test]
fn test_search_limit() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "limit-ch"])
        .assert()
        .success();
    for i in 0..5 {
        cmd(&tmp)
            .args([
                "post",
                "limit-ch",
                "--sender",
                "user",
                "--content",
                &format!("searchable message number {i}"),
            ])
            .assert()
            .success();
    }

    let output = cmd_json(&tmp)
        .args(["search", "--query", "searchable", "--limit", "2"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["results"].as_array().unwrap().len(), 2);
}

#[test]
fn test_search_no_results() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "empty-ch"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "empty-ch",
            "--sender",
            "user",
            "--content",
            "nothing relevant here",
        ])
        .assert()
        .success();

    cmd(&tmp)
        .args(["search", "--query", "nonexistentterm"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No messages found."));
}

#[test]
fn test_search_json_output() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "json-ch"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "json-ch",
            "--sender",
            "tester",
            "--content",
            "unique findable content here",
        ])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["search", "--query", "findable"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 1);
    let results = parsed["results"].as_array().unwrap();
    assert_eq!(results[0]["channel"], "json-ch");
    assert_eq!(results[0]["sender"], "tester");
    assert!(results[0]["content"].as_str().unwrap().contains("findable"));
    assert!(results[0]["timestamp"].as_str().is_some());
    assert!(results[0]["id"].as_str().is_some());
}

// === Channel Archiving ===

#[test]
fn test_archive_channel_excludes_from_list() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "active-ch"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "create", "--name", "to-archive"])
        .assert()
        .success();

    cmd(&tmp)
        .args(["channel", "archive", "to-archive"])
        .assert()
        .success()
        .stdout(predicate::str::contains("archived"));

    let output = cmd_json(&tmp).args(["channel", "list"]).output().unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 1);
    assert_eq!(parsed["channels"][0]["name"], "active-ch");
}

#[test]
fn test_unarchive_channel_reappears_in_list() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "revive-ch"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "archive", "revive-ch"])
        .assert()
        .success();

    let output = cmd_json(&tmp).args(["channel", "list"]).output().unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 0);

    cmd(&tmp)
        .args(["channel", "unarchive", "revive-ch"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unarchived"));

    let output = cmd_json(&tmp).args(["channel", "list"]).output().unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 1);
    assert_eq!(parsed["channels"][0]["name"], "revive-ch");
}

#[test]
fn test_include_archived_shows_all() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "visible"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "create", "--name", "hidden"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "archive", "hidden"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["channel", "list", "--include-archived"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 2);
}

#[test]
fn test_post_to_archived_channel_fails() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "frozen"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "archive", "frozen"])
        .assert()
        .success();

    cmd(&tmp)
        .args([
            "post",
            "frozen",
            "--sender",
            "agent",
            "--content",
            "should fail",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Cannot post to archived channel 'frozen'",
        ));
}

#[test]
fn test_post_to_archived_channel_json_error() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "frozen-json"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "archive", "frozen-json"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args([
            "post",
            "frozen-json",
            "--sender",
            "agent",
            "--content",
            "nope",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert!(
        parsed["error"]
            .as_str()
            .unwrap()
            .contains("Cannot post to archived channel")
    );
}

#[test]
fn test_read_from_archived_channel_works() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "readable"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "readable",
            "--sender",
            "agent",
            "--content",
            "still here",
        ])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "archive", "readable"])
        .assert()
        .success();

    let output = cmd_json(&tmp).args(["read", "readable"]).output().unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 1);
    assert_eq!(parsed["messages"][0]["content"], "still here");
}

#[test]
fn test_archive_unarchive_by_name() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "named-ch"])
        .assert()
        .success();

    let output = cmd_json(&tmp)
        .args(["channel", "archive", "named-ch"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["name"], "named-ch");
    assert_eq!(parsed["archived"], true);

    let output = cmd_json(&tmp)
        .args(["channel", "unarchive", "named-ch"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["name"], "named-ch");
    assert_eq!(parsed["archived"], false);
}

#[test]
fn test_archive_nonexistent_channel_fails() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "archive", "ghost"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// === Wait Command ===

#[test]
fn test_wait_detects_message_with_thread() {
    use std::thread;
    use std::time::Duration;

    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db").to_str().unwrap().to_string();

    // Create channel
    Command::cargo_bin("chat-management")
        .unwrap()
        .args(["--db", &db_path, "channel", "create", "--name", "wait-th"])
        .assert()
        .success();

    // Spawn a thread that will post a message after a short delay
    let db_path_clone = db_path.clone();
    let poster = thread::spawn(move || {
        thread::sleep(Duration::from_millis(800));
        Command::cargo_bin("chat-management")
            .unwrap()
            .args([
                "--db",
                &db_path_clone,
                "post",
                "wait-th",
                "--sender",
                "poster",
                "--content",
                "hello from thread",
            ])
            .assert()
            .success();
    });

    // Run wait — should block until the poster thread posts
    Command::cargo_bin("chat-management")
        .unwrap()
        .args(["--db", &db_path, "wait", "wait-th", "--timeout", "5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello from thread"))
        .stdout(predicate::str::contains("poster"));

    poster.join().unwrap();
}

#[test]
fn test_wait_timeout() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "wait-to"])
        .assert()
        .success();

    cmd(&tmp)
        .args(["wait", "wait-to", "--timeout", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Timeout: no new messages in wait-to after 1 seconds",
        ));
}

#[test]
fn test_wait_nonexistent_channel() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["wait", "ghost-channel", "--timeout", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_wait_archived_channel() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "wait-arch"])
        .assert()
        .success();
    cmd(&tmp)
        .args(["channel", "archive", "wait-arch"])
        .assert()
        .success();

    cmd(&tmp)
        .args(["wait", "wait-arch", "--timeout", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Cannot wait on archived channel"));
}

#[test]
fn test_wait_json_output() {
    use std::thread;
    use std::time::Duration;

    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db").to_str().unwrap().to_string();

    Command::cargo_bin("chat-management")
        .unwrap()
        .args(["--db", &db_path, "channel", "create", "--name", "wait-json"])
        .assert()
        .success();

    let db_path_clone = db_path.clone();
    let poster = thread::spawn(move || {
        thread::sleep(Duration::from_millis(800));
        Command::cargo_bin("chat-management")
            .unwrap()
            .args([
                "--db",
                &db_path_clone,
                "post",
                "wait-json",
                "--sender",
                "json-poster",
                "--content",
                "json wait test",
            ])
            .assert()
            .success();
    });

    let output = Command::cargo_bin("chat-management")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "--output",
            "json",
            "wait",
            "wait-json",
            "--timeout",
            "5",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["sender"], "json-poster");
    assert_eq!(parsed["content"], "json wait test");
    assert!(parsed["id"].as_str().is_some());
    assert!(parsed["timestamp"].as_str().is_some());

    poster.join().unwrap();
}

// === FTS5 Backfill on Upgrade ===

#[test]
fn test_fts5_backfill_existing_messages() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let db_str = db_path.to_str().unwrap();

    // Create old schema (no FTS table, no triggers) and insert messages directly
    {
        let conn = rusqlite::Connection::open(db_str).unwrap();
        conn.execute_batch(
            "CREATE TABLE channels (
                 id            INTEGER PRIMARY KEY AUTOINCREMENT,
                 name          TEXT    NOT NULL,
                 namespace     TEXT    NOT NULL DEFAULT 'default',
                 purpose       TEXT,
                 created_at    TEXT    NOT NULL,
                 message_count INTEGER NOT NULL DEFAULT 0,
                 UNIQUE (name, namespace)
             );

             CREATE TABLE messages (
                 id              TEXT PRIMARY KEY,
                 channel_id      INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
                 sender          TEXT    NOT NULL,
                 content         TEXT    NOT NULL,
                 timestamp       TEXT    NOT NULL,
                 reply_to        TEXT,
                 idempotency_key TEXT
             );

             CREATE INDEX idx_messages_channel_ts ON messages (channel_id, timestamp);
             CREATE INDEX idx_messages_sender ON messages (sender);
             CREATE UNIQUE INDEX idx_messages_idempotency
                 ON messages (idempotency_key) WHERE idempotency_key IS NOT NULL;

             CREATE TABLE mentions (
                 id              INTEGER PRIMARY KEY AUTOINCREMENT,
                 message_id      TEXT    NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                 channel_id      INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
                 mentioned_agent TEXT    NOT NULL,
                 created_at      TEXT    NOT NULL
             );

             CREATE TABLE schema_versions (
                 version    INTEGER PRIMARY KEY,
                 applied_at TEXT    NOT NULL
             );

             INSERT INTO schema_versions (version, applied_at) VALUES (1, '2025-01-01T00:00:00Z');",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO channels (name, namespace, purpose, created_at, message_count) VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["backfill-ch", "default", "test channel", "2025-01-01T00:00:00Z", 3],
        ).unwrap();

        for i in 1..=3 {
            conn.execute(
                "INSERT INTO messages (id, channel_id, sender, content, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    format!("msg-{i}"),
                    1i64,
                    "old-sender",
                    format!("backfill searchterm message {i}"),
                    format!("2025-01-01T00:00:0{i}Z")
                ],
            ).unwrap();
        }
    }

    // Verify .bak file is created
    let bak_path = tmp.path().join("test.db.bak");
    assert!(!bak_path.exists());

    // Run search which triggers Database::open — this should backfill the FTS index
    let output = Command::cargo_bin("chat-management")
        .unwrap()
        .args(["--db", db_str, "--output", "json", "search", "--query", "searchterm"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(bak_path.exists(), "Backup file should be created");

    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["total"], 3);
    let results = parsed["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    assert!(
        results[0]["content"]
            .as_str()
            .unwrap()
            .contains("searchterm")
    );

    // Verify backfill message in stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Backfilled 3 messages into FTS index"),
        "Expected backfill log in stderr, got: {stderr}"
    );
}

// === CSV Output ===

#[test]
fn test_channel_list_csv() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "channel",
            "create",
            "--name",
            "csv-ch",
            "--purpose",
            "CSV test",
        ])
        .assert()
        .success();

    let output = cmd_csv(&tmp).args(["channel", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "id,name,namespace,purpose,message_count");
    assert!(lines[1].contains("csv-ch"));
    assert!(lines[1].contains("CSV test"));
}

#[test]
fn test_channel_create_csv() {
    let tmp = TempDir::new().unwrap();
    let output = cmd_csv(&tmp)
        .args(["channel", "create", "--name", "newcsv"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "id,name,namespace,purpose,message_count");
    assert!(lines[1].contains("newcsv"));
}

#[test]
fn test_read_messages_csv() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "csvread"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "csvread",
            "--sender",
            "alice",
            "--content",
            "hello world",
        ])
        .assert()
        .success();

    let output = cmd_csv(&tmp).args(["read", "csvread"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "id,channel_id,sender,timestamp,content");
    assert!(lines[1].contains("alice"));
    assert!(lines[1].contains("hello world"));
}

#[test]
fn test_search_csv() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args(["channel", "create", "--name", "csvsearch"])
        .assert()
        .success();
    cmd(&tmp)
        .args([
            "post",
            "csvsearch",
            "--sender",
            "bob",
            "--content",
            "unique csv searchterm",
        ])
        .assert()
        .success();

    let output = cmd_csv(&tmp)
        .args(["search", "--query", "searchterm"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "id,channel,sender,timestamp,content");
    assert!(lines[1].contains("bob"));
    assert!(lines[1].contains("unique csv searchterm"));
}

#[test]
fn test_csv_escaping() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .args([
            "channel",
            "create",
            "--name",
            "escapech",
            "--purpose",
            "has, commas",
        ])
        .assert()
        .success();

    let output = cmd_csv(&tmp).args(["channel", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"has, commas\""));
}

#[test]
fn test_output_flag_short_form() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut c = Command::cargo_bin("chat-management").unwrap();
    c.arg("--db")
        .arg(db_path.to_str().unwrap())
        .arg("-o")
        .arg("json")
        .args(["channel", "create", "--name", "shortflag"]);
    let output = c.output().unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["name"], "shortflag");
}
