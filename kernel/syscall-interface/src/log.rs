use crate::SyscallResultDecodeError;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LogArguments {
    pub string_address: usize,
    pub length: usize,
}

#[derive(Debug)]
pub enum LogError {
    InvalidUtf8 { position: usize },
}

#[repr(C)]
#[derive(Clone, Copy)]
union ResponsePayload {
    invalid_utf8_position: usize,

    nothing: (),
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct EncodedResult {
    status_code: u8,
    payload: ResponsePayload,
}

const INVALID_UTF8_STATUS_CODE: u8 = 1;

#[inline]
pub(crate) fn encode_log_result(result: Result<(), LogError>) -> EncodedResult {
    match result {
        Ok(()) => EncodedResult {
            status_code: 0,
            payload: ResponsePayload { nothing: () },
        },
        Err(LogError::InvalidUtf8 { position }) => EncodedResult {
            status_code: INVALID_UTF8_STATUS_CODE,
            payload: ResponsePayload {
                invalid_utf8_position: position,
            },
        },
    }
}

#[inline]
pub(crate) fn decode_log_result(
    result: EncodedResult,
) -> Result<Result<(), LogError>, SyscallResultDecodeError> {
    // SAFETY: all payloads are valid under any bit combination, so can be reinterpreted freely.
    unsafe {
        match (result.status_code, result.payload) {
            (0, _) => Ok(Ok(())),
            (
                INVALID_UTF8_STATUS_CODE,
                ResponsePayload {
                    invalid_utf8_position,
                },
            ) => Ok(Err(LogError::InvalidUtf8 {
                position: invalid_utf8_position,
            })),
            _ => Err(SyscallResultDecodeError::InvalidResultField("status_code")),
        }
    }
}
