use todc_utils::linearizability::WGLChecker;
use todc_utils::specifications::etcd::{history_from_log, EtcdSpecification};

type EtcdChecker = WGLChecker<EtcdSpecification>;

#[macro_export]
macro_rules! etcd_tests {
    ( $($name:ident: $values:expr,)* )=> {
        $(
            #[test]
            fn $name() {
                let (log_number, expected_result) = $values;
                let history = history_from_log(
                    format!("tests/linearizability/etcd/etcd_{}.log", log_number)
                );
                let result = EtcdChecker::is_linearizable(history);
                assert_eq!(result, expected_result);
            }
        )*
    }
}

// For the source of these values, see
// https://github.com/ahorn/linearizability-checker/blob/9b589ae2a8654e1272194b7d9a1644b432a73326/lt.cc#L5400
etcd_tests! {
    test_000: ("000", false),
    test_001: ("001", false),
    test_002: ("002", true),
    test_003: ("003", false),
    test_004: ("004", false),
    test_005: ("005", true),
    test_006: ("006", false),
    test_007: ("007", true),
    test_008: ("008", false),
    test_009: ("009", false),
    test_010: ("010", false),
    test_011: ("011", false),
    test_012: ("012", false),
    test_013: ("013", false),
    test_014: ("014", false),
    test_015: ("015", false),
    test_016: ("016", false),
    test_017: ("017", false),
    test_018: ("018", true),
    test_019: ("019", false),
    test_020: ("020", false),
    test_021: ("021", false),
    test_022: ("022", false),
    test_023: ("023", false),
    test_024: ("024", false),
    test_025: ("025", true),
    test_026: ("026", false),
    test_027: ("027", false),
    test_028: ("028", false),
    test_029: ("029", false),
    test_030: ("030", false),
    test_031: ("031", true),
    test_032: ("032", false),
    test_033: ("033", false),
    test_034: ("034", false),
    test_035: ("035", false),
    test_036: ("036", false),
    test_037: ("037", false),
    test_038: ("038", true),
    test_039: ("039", false),
    test_040: ("040", false),
    test_041: ("041", false),
    test_042: ("042", false),
    test_043: ("043", false),
    test_044: ("044", false),
    test_045: ("045", true),
    test_046: ("046", false),
    test_047: ("047", false),
    test_048: ("048", true),
    test_049: ("049", true),
    test_050: ("050", false),
    test_051: ("051", true),
    test_052: ("052", false),
    test_053: ("053", true),
    test_054: ("054", false),
    test_055: ("055", false),
    test_056: ("056", true),
    test_057: ("057", false),
    test_058: ("058", false),
    test_059: ("059", false),
    test_060: ("060", false),
    test_061: ("061", false),
    test_062: ("062", false),
    test_063: ("063", false),
    test_064: ("064", false),
    test_065: ("065", false),
    test_066: ("066", false),
    test_067: ("067", true),
    test_068: ("068", false),
    test_069: ("069", false),
    test_070: ("070", false),
    test_071: ("071", false),
    test_072: ("072", false),
    test_073: ("073", false),
    test_074: ("074", false),
    test_075: ("075", true),
    test_076: ("076", true),
    test_077: ("077", false),
    test_078: ("078", false),
    test_079: ("079", false),
    test_080: ("080", true),
    test_081: ("081", false),
    test_082: ("082", false),
    test_083: ("083", false),
    test_084: ("084", false),
    test_085: ("085", false),
    test_086: ("086", false),
    test_087: ("087", true),
    test_088: ("088", false),
    test_089: ("089", false),
    test_090: ("090", false),
    test_091: ("091", false),
    test_092: ("092", true),
    test_093: ("093", false),
    test_094: ("094", false),
    // Etcd fails to boot during test case 95
    // test_095: ("095", None),
    test_096: ("096", false),
    test_097: ("097", false),
    test_098: ("098", true),
    test_099: ("099", false),
    test_100: ("100", true),
    test_101: ("101", true),
    test_102: ("102", true),
}
