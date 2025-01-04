// Uncomment this block to pass the first stage
use std::net::UdpSocket;

// All communications in the DNS protocol are carried in a single format called a "message".
// Each message consists of 5 sections: header, question, answer, authority, and an additional space.
// https://en.wikipedia.org/wiki/Domain_Name_System#DNS_message_format
struct DNSMessage {
    header: Header,
    // question: Question,
    // answer: Answer,
    // authority: Authority,
    // additional: &str
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum OpCode {
    Query = 0,
    IQuery = 1,
    Status = 2,
}

impl TryFrom<u8> for OpCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(OpCode::Query),
            1 => Ok(OpCode::IQuery),
            2 => Ok(OpCode::Status),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum ResponseCode {
    NoError = 0,
    FormError = 1,
    ServFail = 2,
    NxDomain = 3,
}

impl TryFrom<u8> for ResponseCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ResponseCode::NoError),
            1 => Ok(ResponseCode::FormError),
            2 => Ok(ResponseCode::ServFail),
            3 => Ok(ResponseCode::NxDomain),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum QueryOrReply {
    Query = 0,
    Reply = 1,
}

impl TryFrom<u8> for QueryOrReply {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(QueryOrReply::Query),
            1 => Ok(QueryOrReply::Reply),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Header {
    // id 2 bytes
    id: u16,
    // Flags section - 2 bytes
    // Indicates if the message is a query (0) or a reply (1)
    header_flags: HeaderFlags,
    // each count 2 bytes
    question_count: u16,
    answer_record_count: u16,
    authority_record_count: u16,
    additional_record_count: u16,
}

#[derive(Debug, Eq, PartialEq)]
struct HeaderFlags {
    qr: QueryOrReply,
    op_code: OpCode,
    authoritative_answer: bool,
    truncation: bool,
    recursion_desired: bool,
    recursion_available: bool,
    response_code: ResponseCode,
}

impl Header {
    fn extract_flags(bytes: &[u8]) -> HeaderFlags {
        // The Flags section is a 2 byte long section consisting of bools from single bits
        // and 2 0.5 byte op codes

        let flags1 = bytes[2];
        let qr = (flags1 >> 7) & 0b1;
        let op_code = (flags1 & 0b0111_1000) >> 3;
        let authoritative_answer = (flags1 & 0b0000_0100) != 0;
        let truncation = (flags1 & 0b0000_0010) != 0;
        let recursion_desired = (flags1 & 0b0000_0001) != 0;

        // Fourth byte contains remaining flags
        let flags2 = bytes[3];
        let recursion_available = (flags2 & 0b1000_0000) != 0;
        // Reserved / unused - assume 0 on serialize
        let _ = (flags2 & 0b0111_0000) >> 4;
        let response_code = flags2 & 0b0000_1111;

        let op_code = OpCode::try_from(op_code).unwrap();
        let response_code = ResponseCode::try_from(response_code).unwrap();
        let qr = QueryOrReply::try_from(qr).unwrap();

        HeaderFlags {
            qr,
            op_code,
            authoritative_answer,
            truncation,
            recursion_desired,
            recursion_available,
            response_code,
        }
    }

    fn new(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() == 12);
        let id = u16::from_be_bytes([bytes[0], bytes[1]]);
        let header_flags = Self::extract_flags(bytes);
        let question_count = u16::from_be_bytes([bytes[4], bytes[5]]);
        let answer_record_count = u16::from_be_bytes([bytes[6], bytes[7]]);
        let authority_record_count = u16::from_be_bytes([bytes[8], bytes[9]]);
        let additional_record_count = u16::from_be_bytes([bytes[10], bytes[11]]);
        Header {
            id,
            header_flags,
            question_count,
            answer_record_count,
            authority_record_count,
            additional_record_count,
        }
    }

    fn to_bytes(&self) -> [u8; 12] {
        let mut bytes = [0u8; 12];

        // Serialize ID (16bits)
        bytes[0..2].copy_from_slice(&self.id.to_be_bytes());

        // Serialize flags (16bits)
        let mut flags: u16 = 0;
        flags |= (self.header_flags.qr.clone() as u16) << 15; // bit 15
        flags |= (self.header_flags.op_code.clone() as u16) << 11; // bit 14-11
        flags |= (self.header_flags.authoritative_answer as u16) << 10; // bit 10
        flags |= (self.header_flags.truncation as u16) << 9; // bit 9
        flags |= (self.header_flags.recursion_desired as u16) << 8; // bit 8
        flags |= (self.header_flags.recursion_available as u16) << 7; // bit 7
        flags |= ((0_u16) & 0x7) << 4; // bit 6-4 (ensure only the lowest 3bits assigned is used)
        flags |= (self.header_flags.response_code.clone() as u16) & 0xF; // bit 3-0 (ensure only the lowest 4bits assigned is used)
        bytes[2..4].copy_from_slice(&flags.to_be_bytes());

        // Serialize counts
        bytes[4..6].copy_from_slice(&self.question_count.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.answer_record_count.to_be_bytes());
        bytes[8..10].copy_from_slice(&self.authority_record_count.to_be_bytes());
        bytes[10..12].copy_from_slice(&self.additional_record_count.to_be_bytes());

        bytes
    }
}

fn main() {
    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0; 512];

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, source)) => {
                println!("Received {} bytes from {}", size, source);
                let filled_buf = &mut buf[..size];
                let (raw_header, _rest) = filled_buf.split_at(12);
                let mut header = Header::new(raw_header);
                println!("Received header:{:?}", header);
                header.header_flags.qr = QueryOrReply::Reply;
                let _message = DNSMessage { header };
                println!("Response header:{:?}", _message.header);
                udp_socket
                    .send_to(&_message.header.to_bytes(), source)
                    .expect("Failed to send response");
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}
