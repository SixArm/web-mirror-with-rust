//! Sitemaps: any XML, gzip, or date text parses without panicking, and
//! decompression never returns more than its limit.
#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use web_mirror::sitemap::from_bytes_into_decompressed_bytes::from_bytes_into_decompressed_bytes;
use web_mirror::sitemap::from_w3c_datetime_str_into_system_time::from_w3c_datetime_str_into_system_time;
use web_mirror::sitemap::from_xml_str_into_sitemap::from_xml_str_into_sitemap;

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    xml: &'a str,
    date: &'a str,
    gzip: &'a [u8],
    limit: u16,
}

fuzz_target!(|input: Input| {
    let _ = from_xml_str_into_sitemap(input.xml);
    let _ = from_w3c_datetime_str_into_system_time(input.date);
    let limit = u64::from(input.limit);
    if let Some(bytes) = from_bytes_into_decompressed_bytes(input.gzip, limit) {
        assert!(bytes.len() as u64 <= limit, "{} > {limit}", bytes.len());
    }
});
