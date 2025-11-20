use juicebox_omega::utils::sanitize_filename;

#[test]
fn test_sanitize_filename() {
    // basic alphanumeric with extension
    assert_eq!(sanitize_filename("hello.txt"), "hello.txt");
    
    // directory traversal attempts
    assert_eq!(sanitize_filename("../hello.txt"), "hello.txt");
    assert_eq!(sanitize_filename("foo/bar.txt"), "foobar.txt");
    assert_eq!(sanitize_filename("/etc/passwd"), "etcpasswd");
    
    // special characters
    assert_eq!(sanitize_filename("hello-world_123.txt"), "hello-world_123.txt");
    assert_eq!(sanitize_filename("hello@world.txt"), "helloworld.txt");
    
    // leading dots
    assert_eq!(sanitize_filename(".hidden"), "hidden");
    assert_eq!(sanitize_filename("..hidden"), "hidden");
}
