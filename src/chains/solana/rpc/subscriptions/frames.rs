// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Parsing of inbound WebSocket frames: notifications, subscribe confirmations and errors.

use super::types::Mention;

/// The JSON-RPC code of a request for a method the endpoint does not serve.
pub(super) const METHOD_NOT_FOUND: i64 = -32601;

/// Parsed form of one inbound WebSocket text frame.
#[derive(Debug, PartialEq)]
pub(super) enum ParsedMessage {
    Mention {
        subscription_id: u64,
        mention: Mention,
    },
    AccountChange {
        subscription_id: u64,
        slot: u64,
    },
    Confirmation {
        request_id: u64,
        subscription_id: u64,
    },
    RpcError {
        request_id: Option<u64>,
        /// The JSON-RPC error code; `METHOD_NOT_FOUND` names an endpoint that does
        /// not serve the method at all.
        code: Option<i64>,
        message: String,
    },
    Ignored,
}

/// Parse one inbound WebSocket text frame. Pure, no I/O.
pub(super) fn parse_message(raw: &str) -> ParsedMessage {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return ParsedMessage::Ignored;
    };

    if v["method"] == "logsNotification" {
        let Some(subscription_id) = v["params"]["subscription"].as_u64() else {
            return ParsedMessage::Ignored;
        };
        let value = &v["params"]["result"]["value"];
        let Some(signature) = value["signature"].as_str() else {
            return ParsedMessage::Ignored;
        };
        return ParsedMessage::Mention {
            subscription_id,
            mention: Mention {
                signature: signature.to_owned(),
                slot: v["params"]["result"]["context"]["slot"].as_u64(),
                failed: !value["err"].is_null(),
            },
        };
    }

    if v["method"] == "accountNotification" {
        let (Some(subscription_id), Some(slot)) = (
            v["params"]["subscription"].as_u64(),
            v["params"]["result"]["context"]["slot"].as_u64(),
        ) else {
            return ParsedMessage::Ignored;
        };
        return ParsedMessage::AccountChange {
            subscription_id,
            slot,
        };
    }

    if v["error"].is_object() {
        return ParsedMessage::RpcError {
            request_id: v["id"].as_u64(),
            code: v["error"]["code"].as_i64(),
            message: v["error"]["message"]
                .as_str()
                .unwrap_or("unknown")
                .to_owned(),
        };
    }

    if let (Some(request_id), Some(subscription_id)) = (v["id"].as_u64(), v["result"].as_u64()) {
        return ParsedMessage::Confirmation {
            request_id,
            subscription_id,
        };
    }

    ParsedMessage::Ignored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_message_notification_success() {
        let raw = r#"{
            "jsonrpc": "2.0",
            "method": "logsNotification",
            "params": {
                "result": {
                    "context": { "slot": 5208469 },
                    "value": {
                        "signature": "5h6xBEauJ3PK6SWCZ1PGjBvj8vDdWG3KpwATGy1ARAXFSDwt8GFXM7W5Ncn16wmqokgpiKRLuS83KUxyZyv2sUYv",
                        "err": null,
                        "logs": [
                            "Program 11111111111111111111111111111111 invoke [1]",
                            "Program 11111111111111111111111111111111 success"
                        ]
                    }
                },
                "subscription": 24040
            }
        }"#;

        match parse_message(raw) {
            ParsedMessage::Mention {
                subscription_id,
                mention,
            } => {
                assert_eq!(subscription_id, 24040);
                assert_eq!(
                    mention.signature,
                    "5h6xBEauJ3PK6SWCZ1PGjBvj8vDdWG3KpwATGy1ARAXFSDwt8GFXM7W5Ncn16wmqokgpiKRLuS83KUxyZyv2sUYv"
                );
                assert_eq!(mention.slot, Some(5208469));
                assert!(!mention.failed);
            }
            other => panic!("expected Mention, got {other:?}"),
        }
    }

    #[test]
    fn parse_message_notification_failed_transaction() {
        let raw = r#"{
            "jsonrpc": "2.0",
            "method": "logsNotification",
            "params": {
                "result": {
                    "context": { "slot": 1 },
                    "value": {
                        "signature": "sig",
                        "err": { "InstructionError": [0, { "Custom": 1 }] },
                        "logs": []
                    }
                },
                "subscription": 1
            }
        }"#;

        match parse_message(raw) {
            ParsedMessage::Mention { mention, .. } => assert!(mention.failed),
            other => panic!("expected Mention, got {other:?}"),
        }
    }

    #[test]
    fn parse_message_confirmation() {
        let raw = r#"{"jsonrpc":"2.0","result":24040,"id":1}"#;
        assert_eq!(
            parse_message(raw),
            ParsedMessage::Confirmation {
                request_id: 1,
                subscription_id: 24040,
            }
        );
    }

    #[test]
    fn parse_message_rpc_error() {
        let raw = r#"{"jsonrpc":"2.0","error":{"code":-32602,"message":"Invalid params"},"id":1}"#;
        match parse_message(raw) {
            ParsedMessage::RpcError {
                request_id,
                code,
                message,
            } => {
                assert_eq!(request_id, Some(1));
                assert_eq!(code, Some(-32602));
                assert_eq!(message, "Invalid params");
            }
            other => panic!("expected RpcError, got {other:?}"),
        }
    }

    #[test]
    fn parse_message_ignored_cases() {
        assert_eq!(parse_message("not json"), ParsedMessage::Ignored);
        assert_eq!(parse_message("{}"), ParsedMessage::Ignored);
    }

    #[test]
    fn an_account_change_carries_its_slot() {
        let raw = r#"{"jsonrpc":"2.0","method":"accountNotification","params":{"result":{"context":{"slot":5199307},"value":{"data":["","base64"],"executable":false,"lamports":33594,"owner":"11111111111111111111111111111111","rentEpoch":635,"space":0}},"subscription":23784}}"#;
        assert_eq!(
            parse_message(raw),
            ParsedMessage::AccountChange {
                subscription_id: 23784,
                slot: 5199307
            }
        );
    }

    #[test]
    fn an_endpoint_without_the_method_answers_method_not_found() {
        let raw = r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"Method 'logsSubscribe' not found"},"id":3}"#;
        assert!(matches!(
            parse_message(raw),
            ParsedMessage::RpcError {
                request_id: Some(3),
                code: Some(METHOD_NOT_FOUND),
                ..
            }
        ));
    }
}
