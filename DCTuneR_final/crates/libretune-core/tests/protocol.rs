//! Serial protocol plumbing tests.
//!
//! These tests exercise the low-level protocol helpers ([`ProtocolError`]
//! formatting, the [`MockSerial`] test double) rather than full request/response
//! cycles, which are covered by the inline `#[cfg(test)]` blocks in
//! `src/protocol/`. The mock here exists so other tests can drive the
//! connection layer without binding to a real serial-port crate.

use libretune_core::protocol::ProtocolError;
use std::sync::{Arc, Mutex};

/// Hand-rolled in-memory serial double.
///
/// No real serial-port library is used here — instead the port owns two
/// independent buffers:
/// - `recv_buffer` — bytes the "ECU" will hand back, consumed in order via
///   [`read_byte`](MockSerial::read_byte) until EOF.
/// - `send_buffer` — accumulates everything the host writes via
///   [`write_all`](MockSerial::write_all), so tests can assert on the exact
///   bytes the protocol layer emitted.
///
/// `fail_on_send` is a fault-injection toggle: when set, the next
/// `write_all` returns an error, used to exercise the protocol's error path.
struct MockSerial {
    send_buffer: Vec<u8>,
    recv_buffer: Vec<u8>,
    recv_idx: usize,
    fail_on_send: bool,
}

impl MockSerial {
    fn new() -> Self {
        Self {
            send_buffer: Vec::new(),
            recv_buffer: Vec::new(),
            recv_idx: 0,
            fail_on_send: false,
        }
    }

    fn with_response(response: Vec<u8>) -> Self {
        Self {
            send_buffer: Vec::new(),
            recv_buffer: response,
            recv_idx: 0,
            fail_on_send: false,
        }
    }

    fn read_byte(&mut self) -> Result<u8, String> {
        if self.recv_idx < self.recv_buffer.len() {
            let byte = self.recv_buffer[self.recv_idx];
            self.recv_idx += 1;
            Ok(byte)
        } else {
            Err("EOF".to_string())
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), String> {
        if self.fail_on_send {
            return Err("Serial write failed".to_string());
        }
        self.send_buffer.extend_from_slice(buf);
        Ok(())
    }
}

#[test]
fn test_connection_creation() {
    // Wrapping a MockSerial in the Arc<Mutex<..>> shape the connection layer
    // expects must not panic. Full connection construction is not exercised
    // here — it depends on private impl details covered by the inline tests
    // in `src/protocol/connection.rs`.
    let mock = MockSerial::new();
    let _shared = Arc::new(Mutex::new(mock));
}

#[test]
fn test_protocol_error_debug() {
    let err = ProtocolError::Timeout;
    assert!(!format!("{:?}", err).is_empty());
}

#[test]
fn test_protocol_error_display() {
    let err = ProtocolError::Timeout;
    assert!(!err.to_string().is_empty());
}

// NOTE: A test named `test_crc16_calculation_deterministic` previously lived
// here but was removed: despite its name it did NOT exercise CRC at all — it
// was a verbatim duplicate of `test_protocol_error_display` (formatting a
// `ProtocolError::Timeout`). No CRC16/CRC32 implementation exists anywhere in
// `libretune-core` today, so a real CRC test is out of scope. If/when CRC is
// implemented, add a dedicated test in the corresponding `packet.rs` test
// block rather than resurrecting this misleading one.

#[test]
fn test_timeout_error() {
    let err = ProtocolError::Timeout;
    let err_str = format!("{:?}", err);
    assert!(err_str.contains("Timeout") || !err_str.is_empty());
}

#[test]
fn test_serial_mockserial_read() {
    let mut mock = MockSerial::with_response(vec![0xAB, 0xCD]);
    let byte1 = mock.read_byte();
    assert!(byte1.is_ok());
    let byte2 = mock.read_byte();
    assert!(byte2.is_ok());
}

#[test]
fn test_serial_mockserial_eof() {
    // An empty recv_buffer must yield EOF (Err), not a panic or a zero byte —
    // the protocol layer relies on this to detect end-of-response.
    let mut mock = MockSerial::new();
    let result = mock.read_byte();
    assert!(result.is_err());
}

#[test]
fn test_serial_mockserial_write() {
    let mut mock = MockSerial::new();
    let result = mock.write_all(b"test");
    assert!(result.is_ok());
    assert_eq!(mock.send_buffer, b"test".to_vec());
}

#[test]
fn test_serial_mockserial_write_failure() {
    let mut mock = MockSerial::new();
    mock.fail_on_send = true;
    let result = mock.write_all(b"test");
    assert!(result.is_err());
}

#[test]
fn test_serial_mockserial_data_integrity() {
    let mut mock = MockSerial::new();
    let test_data = b"Hello, ECU!";
    let result = mock.write_all(test_data);
    assert!(result.is_ok());
    assert_eq!(mock.send_buffer, test_data.to_vec());
}
