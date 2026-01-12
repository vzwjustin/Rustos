// Socket Syscall Implementations for RustOS
// This file contains complete implementations of all network socket syscalls
// To integrate: Replace the TODO implementations in src/process/syscalls.rs starting at line 2044

/// sys_socket - Create socket
fn sys_socket(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
    let domain = args.get(0).copied().unwrap_or(0) as i32;
    let socket_type = args.get(1).copied().unwrap_or(0) as i32;
    let protocol = args.get(2).copied().unwrap_or(0) as i32;

    // Validate domain: AF_INET = 2, AF_INET6 = 10, AF_UNIX = 1
    match domain {
        2 | 10 | 1 => {},
        _ => return SyscallResult::Error(SyscallError::InvalidArgument),
    };

    // Map socket type: SOCK_STREAM = 1, SOCK_DGRAM = 2, SOCK_RAW = 3
    let net_socket_type = match socket_type {
        1 => crate::net::socket::SocketType::Stream,
        2 => crate::net::socket::SocketType::Datagram,
        3 => crate::net::socket::SocketType::Raw,
        _ => return SyscallResult::Error(SyscallError::InvalidArgument),
    };

    // Determine protocol
    let net_protocol = match (socket_type, protocol) {
        (1, 0) | (1, 6) => crate::net::Protocol::TCP,
        (2, 0) | (2, 17) => crate::net::Protocol::UDP,
        (3, 1) => crate::net::Protocol::ICMP,
        (3, 6) => crate::net::Protocol::TCP,
        (3, 17) => crate::net::Protocol::UDP,
        _ => return SyscallResult::Error(SyscallError::InvalidArgument),
    };

    // Check permissions for raw sockets
    if socket_type == 3 {
        if let Some(ctx) = crate::security::get_context(current_pid) {
            if !ctx.is_root() && !crate::security::check_permission(current_pid, "net_raw") {
                return SyscallResult::Error(SyscallError::PermissionDenied);
            }
        } else {
            return SyscallResult::Error(SyscallError::PermissionDenied);
        }
    }

    // Create socket
    let network_stack = crate::net::network_stack();
    match network_stack.create_socket(net_socket_type, net_protocol) {
        Ok(socket_id) => {
            let mut process = match process_manager.get_process(current_pid) {
                Some(p) => p,
                None => {
                    let _ = network_stack.close_socket(socket_id);
                    return SyscallResult::Error(SyscallError::ProcessNotFound);
                }
            };

            let mut next_fd = 3;
            while process.file_descriptors.contains_key(&next_fd) {
                next_fd += 1;
                if next_fd > 65535 {
                    let _ = network_stack.close_socket(socket_id);
                    return SyscallResult::Error(SyscallError::OutOfMemory);
                }
            }

            let fd = super::FileDescriptor {
                fd_type: super::FileDescriptorType::Socket { socket_id },
                flags: 0,
                offset: 0,
            };
            process.file_descriptors.insert(next_fd, fd);
            SyscallResult::Success(next_fd as u64)
        }
        Err(_) => SyscallResult::Error(SyscallError::OutOfMemory),
    }
}

/// sys_bind - Bind socket to address
fn sys_bind(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
    let fd = args.get(0).copied().unwrap_or(0) as u32;
    let addr_ptr = args.get(1).copied().unwrap_or(0);
    let addr_len = args.get(2).copied().unwrap_or(0) as u32;

    if addr_ptr == 0 || addr_len < 8 {
        return SyscallResult::Error(SyscallError::InvalidArgument);
    }

    let process = match process_manager.get_process(current_pid) {
        Some(p) => p,
        None => return SyscallResult::Error(SyscallError::ProcessNotFound),
    };

    let socket_id = match process.file_descriptors.get(&fd) {
        Some(fd_entry) => match &fd_entry.fd_type {
            super::FileDescriptorType::Socket { socket_id } => *socket_id,
            _ => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
        },
        None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
    };

    let mut addr_buffer = vec![0u8; core::cmp::min(addr_len as usize, 128)];
    if let Err(_) = self.copy_from_user(addr_ptr, &mut addr_buffer) {
        return SyscallResult::Error(SyscallError::InvalidAddress);
    }

    let family = u16::from_ne_bytes([addr_buffer[0], addr_buffer[1]]);
    let socket_address = match family {
        2 => {
            // AF_INET
            if addr_buffer.len() < 8 {
                return SyscallResult::Error(SyscallError::InvalidArgument);
            }
            let port = u16::from_be_bytes([addr_buffer[2], addr_buffer[3]]);
            let ip = [addr_buffer[4], addr_buffer[5], addr_buffer[6], addr_buffer[7]];

            if port < 1024 {
                if let Some(ctx) = crate::security::get_context(current_pid) {
                    if !ctx.is_root() && !crate::security::check_permission(current_pid, "net_bind_service") {
                        return SyscallResult::Error(SyscallError::PermissionDenied);
                    }
                } else {
                    return SyscallResult::Error(SyscallError::PermissionDenied);
                }
            }

            crate::net::socket::SocketAddress::ipv4(ip[0], ip[1], ip[2], ip[3], port)
        }
        10 => {
            // AF_INET6
            if addr_buffer.len() < 28 {
                return SyscallResult::Error(SyscallError::InvalidArgument);
            }
            let port = u16::from_be_bytes([addr_buffer[2], addr_buffer[3]]);
            let mut ipv6_addr = [0u8; 16];
            ipv6_addr.copy_from_slice(&addr_buffer[8..24]);

            if port < 1024 {
                if let Some(ctx) = crate::security::get_context(current_pid) {
                    if !ctx.is_root() && !crate::security::check_permission(current_pid, "net_bind_service") {
                        return SyscallResult::Error(SyscallError::PermissionDenied);
                    }
                } else {
                    return SyscallResult::Error(SyscallError::PermissionDenied);
                }
            }

            crate::net::socket::SocketAddress::ipv6(ipv6_addr, port)
        }
        _ => return SyscallResult::Error(SyscallError::InvalidArgument),
    };

    if !socket_address.is_valid() {
        return SyscallResult::Error(SyscallError::InvalidArgument);
    }

    let network_stack = crate::net::network_stack();
    if let Some(mut socket) = network_stack.get_socket(socket_id) {
        match socket.bind(socket_address) {
            Ok(()) => {
                let mut sockets = network_stack.sockets.write();
                sockets.insert(socket_id, socket);
                drop(sockets);
                SyscallResult::Success(0)
            }
            Err(crate::net::NetworkError::AddressInUse) => {
                SyscallResult::Error(SyscallError::ResourceBusy)
            }
            Err(_) => SyscallResult::Error(SyscallError::InvalidAddress),
        }
    } else {
        SyscallResult::Error(SyscallError::InvalidFileDescriptor)
    }
}

/// sys_connect - Connect socket
fn sys_connect(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
    let fd = args.get(0).copied().unwrap_or(0) as u32;
    let addr_ptr = args.get(1).copied().unwrap_or(0);
    let addr_len = args.get(2).copied().unwrap_or(0) as u32;

    if addr_ptr == 0 || addr_len < 8 {
        return SyscallResult::Error(SyscallError::InvalidArgument);
    }

    let process = match process_manager.get_process(current_pid) {
        Some(p) => p,
        None => return SyscallResult::Error(SyscallError::ProcessNotFound),
    };

    let socket_id = match process.file_descriptors.get(&fd) {
        Some(fd_entry) => match &fd_entry.fd_type {
            super::FileDescriptorType::Socket { socket_id } => *socket_id,
            _ => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
        },
        None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
    };

    let mut addr_buffer = vec![0u8; core::cmp::min(addr_len as usize, 128)];
    if let Err(_) = self.copy_from_user(addr_ptr, &mut addr_buffer) {
        return SyscallResult::Error(SyscallError::InvalidAddress);
    }

    let family = u16::from_ne_bytes([addr_buffer[0], addr_buffer[1]]);
    let socket_address = match family {
        2 => {
            if addr_buffer.len() < 8 {
                return SyscallResult::Error(SyscallError::InvalidArgument);
            }
            let port = u16::from_be_bytes([addr_buffer[2], addr_buffer[3]]);
            let ip = [addr_buffer[4], addr_buffer[5], addr_buffer[6], addr_buffer[7]];
            crate::net::socket::SocketAddress::ipv4(ip[0], ip[1], ip[2], ip[3], port)
        }
        10 => {
            if addr_buffer.len() < 28 {
                return SyscallResult::Error(SyscallError::InvalidArgument);
            }
            let port = u16::from_be_bytes([addr_buffer[2], addr_buffer[3]]);
            let mut ipv6_addr = [0u8; 16];
            ipv6_addr.copy_from_slice(&addr_buffer[8..24]);
            crate::net::socket::SocketAddress::ipv6(ipv6_addr, port)
        }
        _ => return SyscallResult::Error(SyscallError::InvalidArgument),
    };

    let network_stack = crate::net::network_stack();
    if let Some(mut socket) = network_stack.get_socket(socket_id) {
        match socket.connect(socket_address) {
            Ok(()) => {
                let mut sockets = network_stack.sockets.write();
                sockets.insert(socket_id, socket);
                drop(sockets);
                SyscallResult::Success(0)
            }
            Err(crate::net::NetworkError::Timeout) => {
                SyscallResult::Error(SyscallError::ResourceBusy)
            }
            Err(_) => SyscallResult::Error(SyscallError::InvalidAddress),
        }
    } else {
        SyscallResult::Error(SyscallError::InvalidFileDescriptor)
    }
}

/// sys_listen - Listen on socket
fn sys_listen(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
    let fd = args.get(0).copied().unwrap_or(0) as u32;
    let backlog = args.get(1).copied().unwrap_or(128) as u32;
    let backlog = core::cmp::min(backlog, 4096);

    let process = match process_manager.get_process(current_pid) {
        Some(p) => p,
        None => return SyscallResult::Error(SyscallError::ProcessNotFound),
    };

    let socket_id = match process.file_descriptors.get(&fd) {
        Some(fd_entry) => match &fd_entry.fd_type {
            super::FileDescriptorType::Socket { socket_id } => *socket_id,
            _ => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
        },
        None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
    };

    let network_stack = crate::net::network_stack();
    if let Some(mut socket) = network_stack.get_socket(socket_id) {
        if socket.local_address.is_none() {
            return SyscallResult::Error(SyscallError::InvalidAddress);
        }
        if socket.socket_type != crate::net::socket::SocketType::Stream {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        match socket.listen(backlog) {
            Ok(()) => {
                let mut sockets = network_stack.sockets.write();
                sockets.insert(socket_id, socket);
                drop(sockets);
                SyscallResult::Success(0)
            }
            Err(_) => SyscallResult::Error(SyscallError::InvalidArgument),
        }
    } else {
        SyscallResult::Error(SyscallError::InvalidFileDescriptor)
    }
}

/// sys_accept - Accept socket connection
fn sys_accept(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
    let fd = args.get(0).copied().unwrap_or(0) as u32;
    let addr_ptr = args.get(1).copied().unwrap_or(0);
    let addrlen_ptr = args.get(2).copied().unwrap_or(0);

    let mut process = match process_manager.get_process(current_pid) {
        Some(p) => p,
        None => return SyscallResult::Error(SyscallError::ProcessNotFound),
    };

    let socket_id = match process.file_descriptors.get(&fd) {
        Some(fd_entry) => match &fd_entry.fd_type {
            super::FileDescriptorType::Socket { socket_id } => *socket_id,
            _ => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
        },
        None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
    };

    let network_stack = crate::net::network_stack();
    if let Some(mut socket) = network_stack.get_socket(socket_id) {
        if socket.state != crate::net::socket::SocketState::Listening {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        match socket.accept() {
            Ok(Some(new_socket_id)) => {
                let mut sockets = network_stack.sockets.write();
                sockets.insert(socket_id, socket);
                let new_socket = sockets.get(&new_socket_id).cloned();
                drop(sockets);

                if let Some(new_socket) = new_socket {
                    let mut next_fd = 3;
                    while process.file_descriptors.contains_key(&next_fd) {
                        next_fd += 1;
                        if next_fd > 65535 {
                            return SyscallResult::Error(SyscallError::OutOfMemory);
                        }
                    }

                    let fd_entry = super::FileDescriptor {
                        fd_type: super::FileDescriptorType::Socket { socket_id: new_socket_id },
                        flags: 0,
                        offset: 0,
                    };
                    process.file_descriptors.insert(next_fd, fd_entry);

                    // Write peer address to user space
                    if addr_ptr != 0 && addrlen_ptr != 0 {
                        if let Some(peer_addr) = new_socket.remote_address {
                            let mut addrlen_buf = [0u8; 4];
                            if self.copy_from_user(addrlen_ptr, &mut addrlen_buf).is_ok() {
                                let max_len = u32::from_ne_bytes(addrlen_buf) as usize;
                                match peer_addr.address {
                                    crate::net::NetworkAddress::IPv4(ip) => {
                                        if max_len >= 8 {
                                            let mut addr_buf = vec![0u8; 8];
                                            addr_buf[0..2].copy_from_slice(&2u16.to_ne_bytes());
                                            addr_buf[2..4].copy_from_slice(&peer_addr.port.to_be_bytes());
                                            addr_buf[4..8].copy_from_slice(&ip);
                                            let _ = self.copy_to_user(addr_ptr, &addr_buf);
                                            let actual_len = 8u32.to_ne_bytes();
                                            let _ = self.copy_to_user(addrlen_ptr, &actual_len);
                                        }
                                    }
                                    crate::net::NetworkAddress::IPv6(ip) => {
                                        if max_len >= 28 {
                                            let mut addr_buf = vec![0u8; 28];
                                            addr_buf[0..2].copy_from_slice(&10u16.to_ne_bytes());
                                            addr_buf[2..4].copy_from_slice(&peer_addr.port.to_be_bytes());
                                            addr_buf[4..8].copy_from_slice(&[0u8; 4]);
                                            addr_buf[8..24].copy_from_slice(&ip);
                                            addr_buf[24..28].copy_from_slice(&[0u8; 4]);
                                            let _ = self.copy_to_user(addr_ptr, &addr_buf);
                                            let actual_len = 28u32.to_ne_bytes();
                                            let _ = self.copy_to_user(addrlen_ptr, &actual_len);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    SyscallResult::Success(next_fd as u64)
                } else {
                    SyscallResult::Error(SyscallError::InvalidAddress)
                }
            }
            Ok(None) => SyscallResult::Error(SyscallError::ResourceBusy),
            Err(_) => SyscallResult::Error(SyscallError::InvalidArgument),
        }
    } else {
        SyscallResult::Error(SyscallError::InvalidFileDescriptor)
    }
}
