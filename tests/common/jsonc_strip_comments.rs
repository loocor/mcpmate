use mcpmate::common::json::strip_comments;

    #[test]
    fn test_strip_comments() {
        // Test single-line comments
        let input = r#"// Comment at start
{
  "key": "value", // inline comment
  // Another comment
  "key2": "value2"
}
"#;
        let result = strip_comments(input);
        println!("Input:\n{}", input);
        println!("Output:\n{}", result);

        // Should be valid JSON after cleaning
        assert!(serde_json::from_str::<serde_json::Value>(&result).is_ok());

        // Test multi-line comments
        let input2 = r#"/* Multi-line
           comment */
{
  "key": "value", /* inline multi */ "key2": "value2"
}
"#;
        let result2 = strip_comments(input2);
        assert!(serde_json::from_str::<serde_json::Value>(&result2).is_ok());

        // Test comments inside strings (should be preserved)
        let input3 = r#"{
  "url": "https://example.com//path",
  "comment": "This // is not a comment"
}"#;
        let result3 = strip_comments(input3);
        let parsed: serde_json::Value = serde_json::from_str(&result3).unwrap();
        assert_eq!(parsed["url"], "https://example.com//path");
        assert_eq!(parsed["comment"], "This // is not a comment");
}
