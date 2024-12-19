use bytes::{Buf, BytesMut};
use owo_colors::OwoColorize;

fn to_avl_ascii(b: u8) -> String {
    match !b.is_ascii_control() {
        true => String::from(b as char).blue().to_string(),
        false => ".".bright_black().to_string(),
    }
}

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
    let group_count = 2;
    let parts_len = per_line / group_count;

    let mut bm = BytesMut::from(bytes);
    let mut out = String::with_capacity(256);

    let mut line_counts = 0;
    while bm.remaining() >= per_line {
        out.push_str(&format!("{:#06x}: ", line_counts * per_line).magenta().to_string());
        let mut ascii_vec = Vec::with_capacity(per_line);
        for _ in 0..parts_len {
            let first_byte = bm.get_u8();
            let second_byte = bm.get_u8();
            out.push_str(format!("{:02x}{:02x} ", first_byte, second_byte).as_ref());
            ascii_vec.push(to_avl_ascii(first_byte));
            ascii_vec.push(to_avl_ascii(second_byte));
        }
        out.push_str("┃ ");
        out.push_str(&ascii_vec.concat());
        out.push('\n');
        line_counts += 1;
    }

    if bm.remaining() > 0 && bm.remaining() < per_line {
        let expect_hex_len = parts_len * (2 * group_count + 1);
        let ordinal = format!("{:#06x}: ", line_counts * per_line).magenta().to_string();
        let mut ascii_vec = Vec::with_capacity(per_line);
        let mut hex_content = String::with_capacity(expect_hex_len);

        while bm.remaining() > 0 {
            if bm.remaining() < 2 {
                let b = bm.get_u8();
                hex_content.push_str(format!("{:02x}", b).as_ref());
                ascii_vec.push(to_avl_ascii(b));
            } else {
                let first_byte = bm.get_u8();
                let second_byte = bm.get_u8();
                hex_content.push_str(format!("{:02x}{:02x} ", first_byte, second_byte).as_ref());
                ascii_vec.push(to_avl_ascii(first_byte));
                ascii_vec.push(to_avl_ascii(second_byte));
            }
        }
        hex_content.extend(vec![' '; expect_hex_len - hex_content.len()]);

        out.push_str(&ordinal);
        out.push_str(&hex_content);
        out.push_str("┃ ");
        out.push_str(&ascii_vec.concat());
        out.push('\n');
    }
    out
}
