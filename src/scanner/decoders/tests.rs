struct ScanRow {
    rank: i32,
    contract_id: i32,
    symbol: String,
    leg: String,
}

#[test]
fn test_decode_scanner_data_top_10() {
    let message = super::ResponseMessage::from("20\03\09000\010\00\0667434000\0ELAB\0STK\0\00\0\0SMART\0USD\0ELAB\0SCM\0SCM\0\0\0\0\01\0689954925\0XTIA\0STK\0\00\0\0SMART\0USD\0XTIA\0SCM\0SCM\0\0\0\0\02\0647805811\0MTEM\0STK\0\00\0\0SMART\0USD\0MTEM\0SCM\0SCM\0\0\0\0\03\0670777621\0SVMH\0STK\0\00\0\0SMART\0USD\0SVMH\0NMS\0NMS\0\0\0\0\04\0324651164\0QUBT\0STK\0\00\0\0SMART\0USD\0QUBT\0SCM\0SCM\0\0\0\0\05\0504717050\0MVST\0STK\0\00\0\0SMART\0USD\0MVST\0SCM\0SCM\0\0\0\0\06\0733727297\0UAVS\0STK\0\00\0\0SMART\0USD\0UAVS\0UAVS\0UAVS\0\0\0\0\07\04815747\0NVDA\0STK\0\00\0\0SMART\0USD\0NVDA\0NMS\0NMS\0\0\0\0\08\076792991\0TSLA\0STK\0\00\0\0SMART\0USD\0TSLA\0NMS\0NMS\0\0\0\0\09\0531212348\0NU\0STK\0\00\0\0SMART\0USD\0NU\0NU\0NU\0\0\0\0\0");

    let scanner_data = super::decode_scanner_data(message).expect("error decoding pnl single");
    assert_eq!(scanner_data.len(), 10, "scanner_data.len()");

    let expected = vec![
        ScanRow {
            rank: 0,
            contract_id: 667434000,
            symbol: "ELAB".to_string(),
            leg: "".to_string(),
        },
        ScanRow {
            rank: 1,
            contract_id: 689954925,
            symbol: "XTIA".to_string(),
            leg: "".to_string(),
        },
        ScanRow {
            rank: 2,
            contract_id: 647805811,
            symbol: "MTEM".to_string(),
            leg: "".to_string(),
        },
        ScanRow {
            rank: 3,
            contract_id: 670777621,
            symbol: "SVMH".to_string(),
            leg: "".to_string(),
        },
        ScanRow {
            rank: 4,
            contract_id: 324651164,
            symbol: "QUBT".to_string(),
            leg: "".to_string(),
        },
        ScanRow {
            rank: 5,
            contract_id: 504717050,
            symbol: "MVST".to_string(),
            leg: "".to_string(),
        },
        ScanRow {
            rank: 6,
            contract_id: 733727297,
            symbol: "UAVS".to_string(),
            leg: "".to_string(),
        },
        ScanRow {
            rank: 7,
            contract_id: 4815747,
            symbol: "NVDA".to_string(),
            leg: "".to_string(),
        },
        ScanRow {
            rank: 8,
            contract_id: 76792991,
            symbol: "TSLA".to_string(),
            leg: "".to_string(),
        },
        ScanRow {
            rank: 9,
            contract_id: 531212348,
            symbol: "NU".to_string(),
            leg: "".to_string(),
        },
    ];

    for i in 0..10 {
        assert_eq!(scanner_data[i].rank, expected[i].rank, "scanner_data[{}].rank", i);
        assert_eq!(
            scanner_data[i].contract_details.contract.contract_id, expected[i].contract_id,
            "scanner_data[{}].contract_id",
            i
        );
        assert_eq!(
            scanner_data[i].contract_details.contract.symbol, expected[i].symbol,
            "scanner_data[{}].symbol",
            i
        );
        assert_eq!(scanner_data[i].leg, expected[i].leg, "scanner_data[{}].leg", i);
    }
}

#[test]
fn test_decode_scanner_data_complex_orders() {
    let message = super::ResponseMessage::from("20\03\09000\050\00\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0738758309|1,738758426|-1\01\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0621537214|1,621537279|-1\02\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0734450979|1,734451081|-3,734451143|2\03\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0682678233|1,736727533|-1\04\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0739480291|1,740950238|-1\05\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0265598|100,584718266|-1,584719753|1\06\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0584718405|1,682678110|-1\07\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0682678233|1,736727485|1\08\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0739480710|1,740950685|-1\09\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0733172883|1,737589816|-1\010\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0265598|100,584718324|-1,584719827|1\011\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0592413828|1,592413865|-1\012\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0621535193|1,621535234|-1\013\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0733171244|2,739480710|-1\014\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0584718445|1,592413922|-1\015\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0682681481|1,739480710|-1\016\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0621535359|1,682678233|-1\017\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0584718476|1,592413501|-1\018\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0736727485|1,740950285|-1\019\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0733172408|1,733172961|-1\020\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0606138777|1,606138801|-1\021\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0736727912|1,736727939|-1\022\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0621535147|1,621535193|-1\023\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0733172883|11,739480710|-6\024\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0265598|100,733171197|-1,733173027|1\025\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0675816239|1,675816415|-1\026\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0740950238|1,740950345|-1\027\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0682678448|1,737588302|-1\028\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0265598|100,584718300|-1,584719802|1\029\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0682681585|1,733173027|-1\030\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0682678233|1,733171197|-1\031\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0621535193|1,740950238|-1\032\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0739480332|1,740950285|-1\033\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0682678233|1,736727485|-1\034\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0734451013|1,734451143|-1\035\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0739480660|1,739480710|-1\036\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0681166346|1,681166531|-1\037\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0682678110|1,722754599|-1\038\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0265598|100,621535037|-1\039\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0722765565|1,737588171|-1\040\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0682678162|1,740111939|-1\041\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0682678233|1,682681530|-1,736727485|-1,736727912|1\042\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0736727485|1,736727533|-1\043\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0618335050|1,682677693|-1\044\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0733170765|1,733171140|-1,734450778|-1,734450868|-1\045\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0722755002|2,722755094|-1\046\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0734452761|1,734452869|-1\047\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0682678216|1,733171140|-1\048\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0682681481|1,736727912|-1\049\028812380\0AAPL\0BAG\0\00\0\0SMART\0USD\0AAPL\0COMB\0COMB\0\0\0\0621537279|1,738758546|-1\0");

    let scanner_data = super::decode_scanner_data(message).expect("error decoding pnl single");
    assert_eq!(scanner_data.len(), 50, "scanner_data.len()");

    let expected = vec![
        ScanRow {
            rank: 0,
            contract_id: 28812380,
            symbol: "AAPL".to_string(),
            leg: "738758309|1,738758426|-1".to_string(),
        },
        ScanRow {
            rank: 1,
            contract_id: 28812380,
            symbol: "AAPL".to_string(),
            leg: "621537214|1,621537279|-1".to_string(),
        },
        ScanRow {
            rank: 2,
            contract_id: 28812380,
            symbol: "AAPL".to_string(),
            leg: "734450979|1,734451081|-3,734451143|2".to_string(),
        },
        ScanRow {
            rank: 3,
            contract_id: 28812380,
            symbol: "AAPL".to_string(),
            leg: "682678233|1,736727533|-1".to_string(),
        },
        ScanRow {
            rank: 4,
            contract_id: 28812380,
            symbol: "AAPL".to_string(),
            leg: "739480291|1,740950238|-1".to_string(),
        },
    ];

    for i in 0..5 {
        assert_eq!(scanner_data[i].rank, expected[i].rank, "scanner_data[{}].rank", i);
        assert_eq!(
            scanner_data[i].contract_details.contract.contract_id, expected[i].contract_id,
            "scanner_data[{}].contract_id",
            i
        );
        assert_eq!(
            scanner_data[i].contract_details.contract.symbol, expected[i].symbol,
            "scanner_data[{}].symbol",
            i
        );
        assert_eq!(scanner_data[i].leg, expected[i].leg, "scanner_data[{}].leg", i);
    }
}
