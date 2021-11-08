use atat::{ComQueue, DefaultDigester, DefaultUrcMatcher, DigestResult, Digester, IngressManager};
use bbqueue::BBBuffer;
use criterion::{criterion_group, criterion_main, Criterion};
use heapless::spsc::Queue;

pub fn ingress_real(c: &mut Criterion) {
    let input_data = heapless::Vec::<u8, 4096>::from_slice(b"AT\r\r\nOK\r\nAT\r\r\nOK\r\nAT\r\r\nOK\r\nAT+CPIN?\r\r\n+CPIN: READY\r\n\r\nOK\r\nAT+CMEE=1\r\r\nOK\r\nAT&C1\r\r\nOK\r\nAT&D0\r\r\nOK\r\nAT+UPSV=0\r\r\nOK\r\nAT+UDCONF=1,1\r\r\nOK\r\nAT&K3\r\r\nOK\r\nAT+UMWI=0\r\r\nOK\r\nAT+CTZU=1\r\r\nOK\r\nAT+CFUN=1,0\r\r\nOK\r\nAT+CGEREP=0\r\r\nOK\r\nAT+CREG=2\r\r\nOK\r\nAT+CGREG=2\r\r\nOK\r\nAT+CEREG=2\r\r\nOK\r\nAT+COPS?\r\r\n+COPS: 0\r\n\r\nOK\r\nAT+CREG?\r\r\n+CREG: 2,0\r\n\r\nOK\r\nAT+CGREG?\r\r\n+CGREG: 2,4\r\n\r\nOK\r\nAT+CEREG?\r\r\n+CEREG: 2,0\r\n\r\nOK\r\nAT+UGPIOC=22,2\r\r\nOK\r\nAT+UGPIOC=21,0,1\r\r\nOK\r\nAT+CFUN=0,0\r\r\nOK\r\n\r\n+CGREG: 0\r\n\r\n+CEREG: 0\r\n\r\n+CEREG: 0\r\nAT+CGDCONT=1,\"IP\",\"em\"\r\r\nOK\r\nAT+UAUTHREQ=1,3,\"\",\"\"\r\r\nOK\r\nAT+CFUN=1,0\r\r\nOK\r\n\r\n+CREG: 5,\"9D6F\",\"01B8DCC8\",2\r\n\r\n+CGREG: 2\r\n\r\n+CEREG: 4\r\n\r\n+CGREG: 5,\"9D6F\",\"01B8DCC8\",2,\"03\"\r\n\r\n+CEREG: 4\r\nAT+CGATT?\r\r\n+CGATT: 1\r\n\r\nOK\r\nAT+CGACT?\r\r\n+CGACT: 1,0\r\n\r\nOK\r\nAT+CGACT=1,1\r\r\nOK\r\nAT+CGATT?\r\r\n+CGATT: 1\r\n\r\nOK\r\nAT+CGACT?\r\r\n+CGACT: 1,1\r\n\r\nOK\r\nAT+UPSD=1,100\r\r\n+UPSD: 1,100,8\r\n\r\nOK\r\nAT+UPSD=1,100,1\r\r\nOK\r\nAT+UPSND=1,8\r\r\n+UPSND: 1,8,0\r\n\r\nOK\r\nAT+UPSND=1,8\r\r\n+UPSND: 1,8,0\r\n\r\nOK\r\nAT+UPSDA=1,3\r\r\nOK\r\n\r\n+UUPSDA: 0,\"100.92.188.240\"\r\nAT+USECMNG=0,0,\"root_ca\",1011\r>\r\n+USECMNG: 0,0,\"root_ca\",\"173574af7b611cebf4f93ce2ee40f9a2\"\r\n\r\nOK\r\nAT+USECPRF=0,3,\"root_ca\"\r\r\nOK\r\nAT+USECMNG=0,1,\"cert\",861\r>\r\n+USECMNG: 0,1,\"cert\",\"f9770848757a7731dc873dba4381d7c8\"\r\n\r\nOK\r\nAT+USECPRF=0,5,\"cert\"\r\r\nOK\r\nAT+USECMNG=0,2,\"priv_key\",1679\r>\r\n+USECMNG: 0,2,\"priv_key\",\"32679253b7bc6e7b4b8c47c9b890dfcf\"\r\n\r\nOK\r\nAT+USECPRF=0,6,\"priv_key\"\r\r\nOK\r\nAT+USECPRF=0,0,3\r\r\nOK\r\nAT+USECPRF=0,2,0\r\r\nOK\r\nAT+USECPRF=0,4,\"a3f8k0ccx04zas-ats.iot.eu-west-1.amazonaws.com\"\r\r\nOK\r\nAT+USOCR=17\r\r\n+USOCR: 0\r\n\r\nOK\r\nAT+USOST=0,\"162.159.200.1\",123,48\r\r\n@\r\n+USOST: 0,48\r\n\r\nOK\r\n\r\n+UUSORD: 0,48\r\nAT+USORF=0,48\r\r\n+USORF: 0,\"162.159.200.1\",123,48,\"240300E70000048B0000001A0A3408F4E50A733A1D3516A60000000000000000E50A739BA97F51F0E50A739BA984AD75\"\r\n\r\nOK\r\nAT+USOCL=0\r\r\nOK\r\nAT+CCLK?\r\r\n+CCLK: \"21/10/08,09:36:59+08\"\r\n\r\nOK\r\nAT+UDNSRN=0,\"a3f8k0ccx04zas-ats.iot.eu-west-1.amazonaws.com\"\r\r\n+UDNSRN: \"54.171.12.213\"\r\n\r\nOK\r\nAT+USOCR=6\r\r\n+USOCR: 0\r\n\r\nOK\r\nAT+USOSEC=0,1,0\r\r\nOK\r\nAT+USOCO=0,\"54.171.12.213\",8883\r\r\nOK\r\nAT+USOWR=0,30\r\r\n@\r\n+USOWR: 0,30\r\n\r\nOK\r\n\r\n+UUSORD: 0,4\r\nAT+USORD=0,4\r\r\n+USORD: 0,4,\"20020000\"\r\n\r\nOK\r\nAT+USOWR=0,196\r\r\n@\r\n+USOWR: 0,196\r\n\r\nOK\r\nAT+USORD=0,4\r\r\n+USORD: 0,0,\"\"\r\n\r\nOK\r\nAT+USOWR=0,52\r\r\n@\r\n+USOWR: 0,52\r\n\r\nOK\r\n\r\n+UUSORD: 0,5\r\nAT+USORD=0,0\r\r\n+USORD: 0,5\r\n\r\nOK\r\nAT+USOWR=0,85\r\r\n@\r\n+USOWR: 0,85\r\n\r\nOK\r\nAT+USORD=0,5\r\r\n+USORD: 0,5,\"9003000201\"\r\n\r\nOK\r\n\r\n+UUSORD: 0,4\r\nAT+USORD=0,4\r\r\n+USORD: 0,4,\"40020003\"\r\n\r\nOK\r\n\r\n+UUSORD: 0,115\r\nAT+USORD=0,115\r\r\n+USORD: 0,115,\"30710034246177732F7468696E67732F373638653861613838623837323330312F6A6F62732F246E6578742F6765742F61636365707465647B22636C69656E74546F6B656E223A22303A37363865386161383862383732333031222C2274696D657374616D70223A313633333637383632357D\"\r\n\r\nOK\r\nAT+USORD=0,115\r\r\n+USORD: 0,0,\"\"\r\n\r\nOK\r\nAT+USORD=0,0\r\r\n+USORD: 0,0\r\n\r\nOK\r\nAT+USOWR=0,425\r\r\n@\r\n+USOWR: 0,425\r\n\r\nOK\r\n").unwrap();

    static mut RES_Q: BBBuffer<8128> = BBBuffer::new();
    let (res_p, res_c) = unsafe { RES_Q.try_split_framed().unwrap() };

    static mut URC_Q: BBBuffer<4096> = BBBuffer::new();
    let (urc_p, _urc_c) = unsafe { URC_Q.try_split_framed().unwrap() };

    static mut COM_Q: ComQueue = Queue::new();
    let (_com_p, com_c) = unsafe { COM_Q.split() };

    let mut ingress = IngressManager::<_, _, 4096, 8128, 4096>::with_customs(
        res_p,
        urc_p,
        com_c,
        DefaultUrcMatcher::default(),
        DefaultDigester::default(),
    );

    c.bench_function("ingress real", |b| {
        b.iter(|| {
            ingress.write(&input_data);
            loop {
                ingress.digest();
                if ingress.is_empty() {
                    break;
                }
            }
        })
    });
}

criterion_group!(benches, ingress_real);

criterion_main!(benches);
