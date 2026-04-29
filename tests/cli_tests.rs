use assert_cmd::Command;
use predicates::prelude::*;
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
    cmd.arg("--db").arg(db_path.to_str().unwrap()).arg("--json");
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
    assert!(parsed["id"].as_str().unwrap().len() > 0);
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
