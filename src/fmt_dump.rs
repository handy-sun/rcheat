use bytes::{Buf, BytesMut};

pub fn dump_to_dec_content(bytes: &[u8]) -> String {
    // for test temp
    let per_line = 16;
    let parts_len = 8;
    let mut bm = BytesMut::from(bytes);
    let mut out = String::with_capacity(256);

    let mut line_counts = 0;
    while bm.remaining() > 0 {
        out.push_str(format!("{:#06x}: ", line_counts * per_line).as_ref());

        if bm.remaining() < per_line {
            out.push_str(format!("{:>3?}  ", bm.get(0..parts_len).unwrap()).as_ref());
            out.push_str(format!("{:>3?}\n", bm.get(parts_len..bm.remaining()).unwrap()).as_ref());
            break;
        }
        out.push_str(format!("{:>3?}  ", bm.get(0..parts_len).unwrap()).as_ref());
        out.push_str(format!("{:>3?}\n", bm.get(parts_len..per_line).unwrap()).as_ref());
        bm.advance(per_line);
        line_counts += 1;
    }
    out
}

pub fn dump_to_hex_content(bytes: &[u8]) -> String {
    let per_line = 16;
    let parts_len = 8;

    let mut bm = BytesMut::from(bytes);
    let mut out = String::with_capacity(256);

    let mut line_counts = 0;
    while bm.remaining() > 0 {
        out.push_str(format!("{:#06x}: ", line_counts * per_line).as_ref());

        if bm.remaining() < per_line {
            while bm.remaining() > 0 {
                if bm.remaining() < 2 {
                    out.push_str(format!("{:02x}", bm.get_u8()).as_ref());
                    // break;
                } else {
                    let first_byte = bm.get_u8();
                    let second_byte = bm.get_u8();
                    out.push_str(format!("{:02x}{:02x} ", first_byte, second_byte).as_ref());
                }
            }
            out.push('\n');
            break;
        }

        for _ in 0..parts_len {
            let first_byte = bm.get_u8();
            let second_byte = bm.get_u8();
            out.push_str(format!("{:02x}{:02x} ", first_byte, second_byte).as_ref());
        }
        out.push('\n');
        line_counts += 1;
    }
    out
}
