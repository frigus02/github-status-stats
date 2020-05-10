#[cfg(not(debug_assertions))]
use crate::{HEADER_PARENT_SPAN_ID, HEADER_TRACE_ID};
use tonic::Request;

#[cfg(debug_assertions)]
pub fn request<T>(message: T) -> Request<T> {
    Request::new(message)
}

#[cfg(not(debug_assertions))]
pub fn request<T>(message: T) -> Request<T> {
    let mut request = Request::new(message);
    if let Ok((trace_id, span_id)) = tracing_honeycomb::current_dist_trace_ctx() {
        let mut metadata = request.metadata_mut();
        metadata.insert(HEADER_TRACE_ID, trace_id.parse().unwrap());
        metadata.insert(HEADER_PARENT_SPAN_ID, span_id.parse().unwrap());
    }
    request
}
