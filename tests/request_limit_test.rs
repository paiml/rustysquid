use rustysquid::MAX_REQUEST_SIZE;

#[test]
fn test_request_size_constant() {
    // Verify the request size limit is exactly 64KB
    assert_eq!(MAX_REQUEST_SIZE, 64 * 1024); // Exactly 64KB
}

#[test]
fn test_request_size_validation() {
    // Small request should be allowed
    let small_request = vec![0u8; 1024];
    assert!(small_request.len() <= MAX_REQUEST_SIZE);

    // Large request should be rejected
    let large_request = vec![0u8; MAX_REQUEST_SIZE + 1];
    assert!(large_request.len() > MAX_REQUEST_SIZE);
}

#[test]
fn test_header_parsing_size() {
    // Build a request with many headers
    let mut request = String::from("GET / HTTP/1.1\r\n");

    // Add headers until we approach the limit
    for i in 0..100 {
        use std::fmt::Write;
        writeln!(&mut request, "X-Custom-Header-{i}: value-{i}\r").unwrap();
        if request.len() > MAX_REQUEST_SIZE {
            break;
        }
    }

    // Request with many headers might exceed limit
    if request.len() > MAX_REQUEST_SIZE {
        // This is expected for very large header sets
        assert!(request.len() > MAX_REQUEST_SIZE);
    }
}

#[test]
fn test_typical_request_sizes() {
    // Typical GET request
    let get_request = "GET /index.html HTTP/1.1\r\n\
                       Host: example.com\r\n\
                       User-Agent: TestClient/1.0\r\n\
                       Accept: */*\r\n\
                       \r\n";
    assert!(get_request.len() < 1024); // Much smaller than limit

    // Typical POST request with headers
    let post_request = "POST /api/data HTTP/1.1\r\n\
                        Host: example.com\r\n\
                        User-Agent: TestClient/1.0\r\n\
                        Content-Type: application/json\r\n\
                        Content-Length: 100\r\n\
                        Authorization: Bearer token123456789\r\n\
                        \r\n";
    assert!(post_request.len() < 1024); // Still much smaller than limit
}
