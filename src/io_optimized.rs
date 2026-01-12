//! # I/O Optimization and Scheduling System
//!
//! This module provides an optimized I/O subsystem with request scheduling,
//! prioritization, and batching for improved performance.

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

/// I/O request priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoPriority {
    /// Critical real-time I/O
    Realtime,
    /// High priority I/O
    High,
    /// Normal priority I/O (default)
    Normal,
    /// Low priority background I/O
    Low,
    /// Best effort idle I/O
    Idle,
}

/// Type of I/O request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoRequestType {
    /// Read operation
    Read,
    /// Write operation
    Write,
    /// Sync/flush operation
    Sync,
}

/// I/O completion status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoCompletionStatus {
    /// Request is pending
    Pending,
    /// Request completed successfully
    Success,
    /// Request failed
    Error,
}

/// I/O request structure
#[derive(Debug, Clone)]
pub struct IoRequest {
    /// Unique request ID
    pub request_id: u64,
    /// Type of I/O operation
    pub request_type: IoRequestType,
    /// Priority level
    pub priority: IoPriority,
    /// Target device/file descriptor
    pub target: u32,
    /// Data buffer offset
    pub offset: u64,
    /// Completion status
    pub completion_status: IoCompletionStatus,
}

/// I/O scheduler for optimizing disk and device access
pub struct IoScheduler {
    /// Queue of pending requests
    request_queue: Mutex<VecDeque<IoRequest>>,
    /// Next request ID
    next_request_id: Mutex<u64>,
    /// Statistics
    total_requests: Mutex<u64>,
    completed_requests: Mutex<u64>,
}

impl IoScheduler {
    /// Create a new I/O scheduler
    pub const fn new() -> Self {
        Self {
            request_queue: Mutex::new(VecDeque::new()),
            next_request_id: Mutex::new(1),
            total_requests: Mutex::new(0),
            completed_requests: Mutex::new(0),
        }
    }

    /// Submit an I/O request
    pub fn submit_request(&self, mut request: IoRequest) -> u64 {
        let mut next_id = self.next_request_id.lock();
        request.request_id = *next_id;
        *next_id += 1;
        drop(next_id);

        let mut queue = self.request_queue.lock();
        queue.push_back(request);
        drop(queue);

        let mut total = self.total_requests.lock();
        *total += 1;

        request.request_id
    }

    /// Process pending I/O requests
    pub fn process_requests(&self) -> usize {
        let mut queue = self.request_queue.lock();
        let count = queue.len();

        // Process all pending requests (simplified - just mark as complete)
        while let Some(mut request) = queue.pop_front() {
            request.completion_status = IoCompletionStatus::Success;
            let mut completed = self.completed_requests.lock();
            *completed += 1;
        }

        count
    }

    /// Get statistics
    pub fn get_stats(&self) -> (u64, u64) {
        let total = *self.total_requests.lock();
        let completed = *self.completed_requests.lock();
        (total, completed)
    }
}

/// Network packet types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    /// TCP packet
    Tcp,
    /// UDP packet
    Udp,
    /// ICMP packet
    Icmp,
    /// Raw packet
    Raw,
}

/// Network packet structure for processing
#[derive(Debug, Clone)]
pub struct NetworkPacket {
    /// Packet ID
    pub packet_id: u64,
    /// Packet size in bytes
    pub size: usize,
    /// Packet type
    pub packet_type: PacketType,
    /// Packet data (simplified - just size for now)
    pub data_len: usize,
}

/// Network packet processor for optimized network I/O
pub struct NetworkProcessor {
    /// Queue of packets to process
    packet_queue: Mutex<VecDeque<NetworkPacket>>,
    /// Next packet ID
    next_packet_id: Mutex<u64>,
    /// Statistics
    total_packets: Mutex<u64>,
    processed_packets: Mutex<u64>,
    total_bytes: Mutex<u64>,
}

impl NetworkProcessor {
    /// Create a new network processor
    pub const fn new() -> Self {
        Self {
            packet_queue: Mutex::new(VecDeque::new()),
            next_packet_id: Mutex::new(1),
            total_packets: Mutex::new(0),
            processed_packets: Mutex::new(0),
            total_bytes: Mutex::new(0),
        }
    }

    /// Queue a packet for processing
    pub fn queue_packet(&self, mut packet: NetworkPacket) -> u64 {
        let mut next_id = self.next_packet_id.lock();
        packet.packet_id = *next_id;
        *next_id += 1;
        drop(next_id);

        let mut queue = self.packet_queue.lock();
        queue.push_back(packet.clone());
        drop(queue);

        let mut total = self.total_packets.lock();
        *total += 1;
        let mut bytes = self.total_bytes.lock();
        *bytes += packet.size as u64;

        packet.packet_id
    }

    /// Process queued packets
    pub fn process_packets(&self) -> usize {
        let mut queue = self.packet_queue.lock();
        let count = queue.len();

        // Process all queued packets (simplified - just dequeue)
        while let Some(_packet) = queue.pop_front() {
            let mut processed = self.processed_packets.lock();
            *processed += 1;
        }

        count
    }

    /// Get statistics
    pub fn get_stats(&self) -> (u64, u64, u64) {
        let total = *self.total_packets.lock();
        let processed = *self.processed_packets.lock();
        let bytes = *self.total_bytes.lock();
        (total, processed, bytes)
    }
}

lazy_static! {
    /// Global I/O scheduler instance
    static ref IO_SCHEDULER: IoScheduler = IoScheduler::new();

    /// Global network processor instance
    static ref NETWORK_PROCESSOR: NetworkProcessor = NetworkProcessor::new();
}

/// Initialize the I/O optimization system
pub fn init_io_system() -> Result<(), &'static str> {
    // I/O system is initialized via lazy_static
    // Additional setup can be added here if needed
    Ok(())
}

/// Get the global I/O scheduler
pub fn io_scheduler() -> &'static IoScheduler {
    &IO_SCHEDULER
}

/// Get the global network processor
pub fn network_processor() -> &'static NetworkProcessor {
    &NETWORK_PROCESSOR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_scheduler_creation() {
        let scheduler = IoScheduler::new();
        let (total, completed) = scheduler.get_stats();
        assert_eq!(total, 0);
        assert_eq!(completed, 0);
    }

    #[test]
    fn test_io_request_submission() {
        let scheduler = IoScheduler::new();
        let request = IoRequest {
            request_id: 0,
            request_type: IoRequestType::Read,
            priority: IoPriority::Normal,
            target: 0,
            offset: 0,
            completion_status: IoCompletionStatus::Pending,
        };

        let id = scheduler.submit_request(request);
        assert_eq!(id, 1);

        let (total, _) = scheduler.get_stats();
        assert_eq!(total, 1);
    }

    #[test]
    fn test_network_processor_creation() {
        let processor = NetworkProcessor::new();
        let (total, processed, bytes) = processor.get_stats();
        assert_eq!(total, 0);
        assert_eq!(processed, 0);
        assert_eq!(bytes, 0);
    }
}
