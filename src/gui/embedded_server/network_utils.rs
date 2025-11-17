// Network utility functions

use std::net::{IpAddr, Ipv4Addr};

/// Get the local IP address of the machine
pub fn get_local_ip() -> Option<IpAddr> {
    // Try to get the local IP by connecting to a public DNS server
    // This doesn't actually send any data, just determines which interface would be used
    use std::net::{TcpStream, SocketAddr};
    
    // Try to connect to Google's DNS (8.8.8.8:80)
    // This will tell us which network interface would be used
    if let Ok(stream) = TcpStream::connect("8.8.8.8:80") {
        if let Ok(local_addr) = stream.local_addr() {
            return Some(local_addr.ip());
        }
    }
    
    // Fallback: try to get from network interfaces
    get_local_ip_from_interfaces()
}

/// Get local IP from network interfaces
fn get_local_ip_from_interfaces() -> Option<IpAddr> {
    use std::net::UdpSocket;
    
    // Create a UDP socket and connect to a public address
    // This doesn't send any data, just determines the local address
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return Some(addr.ip());
            }
        }
    }
    
    // Final fallback: return localhost
    Some(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
}

/// Get all local IP addresses
pub fn get_all_local_ips() -> Vec<IpAddr> {
    let mut ips = Vec::new();
    
    // Add localhost
    ips.push(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    
    // Try to get the primary local IP
    if let Some(ip) = get_local_ip() {
        if !ips.contains(&ip) {
            ips.push(ip);
        }
    }
    
    ips
}

/// Format IP address for display
pub fn format_server_address(ip: &IpAddr, port: u16) -> String {
    match ip {
        IpAddr::V4(ipv4) => format!("http://{}:{}", ipv4, port),
        IpAddr::V6(ipv6) => format!("http://[{}]:{}", ipv6, port),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_local_ip() {
        let ip = get_local_ip();
        assert!(ip.is_some());
        println!("Local IP: {:?}", ip);
    }
    
    #[test]
    fn test_get_all_local_ips() {
        let ips = get_all_local_ips();
        assert!(!ips.is_empty());
        println!("All local IPs: {:?}", ips);
    }
    
    #[test]
    fn test_format_server_address() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        let addr = format_server_address(&ip, 3000);
        assert_eq!(addr, "http://192.168.1.100:3000");
    }
}
