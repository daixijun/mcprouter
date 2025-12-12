#[cfg(test)]
mod tests {
    use super::ServiceTransport;

    #[test]
    fn test_service_transport_parsing() {
        // 测试正确的解析
        assert_eq!("stdio".parse::<ServiceTransport>().unwrap(), ServiceTransport::Stdio);
        assert_eq!("http".parse::<ServiceTransport>().unwrap(), ServiceTransport::Http);

        // 测试错误的解析
        assert!("invalid".parse::<ServiceTransport>().is_err());
    }

    #[test]
    fn test_service_transport_display() {
        assert_eq!(ServiceTransport::Stdio.to_string(), "stdio");
        assert_eq!(ServiceTransport::Http.to_string(), "http");
    }
}