use std::env;
use std::fs::File;
extern crate hound;
extern crate vcd;

use vcd::{TimescaleUnit,Value};

fn usage() {
    println!("usage wav2aes in_file.wav");
    panic!("usage ");
}
fn crackargs_for_filename() -> String {
    if env::args().count() < 2 {
        usage();
    }
    let argout = env::args().nth(1).unwrap();
    argout
}

/// s/wav/vcd
fn outfilename() -> String {
    let infilename = crackargs_for_filename();
    let outfilename = infilename.replace (".wav", ".vcd");
    
    outfilename
}

fn createfile( filename:String ) -> File {
	let fle = File::create ( filename );
	fle.unwrap()
}

const aes_holdoff:u64 = 123;


fn main() {
	let preamble_x:[bool;8]=[true,true,true,false,false,false,true,false];
	let preamble_y:[bool;8]=[true,true,true,false,false,true,false,false];
	let preamble_z:[bool;8]=[true,true,true,false,true,false,false,false];
	let preambles=[preamble_x, preamble_y, preamble_z];

    let mut houndread = hound::WavReader::open(crackargs_for_filename()).unwrap();

    let typspec = houndread.spec();
    println!("infilespec: {:?}", typspec);
    let odo: Vec<i16> = houndread.samples().map(|r| r.unwrap()).collect(); // I hat u

    let mut outfilebuffer = createfile ( outfilename ( )   ); // derive outfilename from infilename
    let mut writr = vcd::Writer::new(  &mut outfilebuffer );
    writr.timescale ( 1, TimescaleUnit::NS ).expect("timescale");
    writr.add_module ( "top").expect("modue");
    let lrclock = writr.add_wire (1,"lr_clock").expect("lrwire");
    let bitclock = writr.add_wire (1,"bit_clock").expect("bitwire");
    let aesout = writr.add_wire (1,"aes").expect("aeswire");
    let noamiout = writr.add_wire (1,"noami").expect("aeswire");
    writr.upscope().expect("us");
    writr.enddefinitions().expect("ed");
    
    let ns_per_sample:f64 =  (0.50 / (typspec.sample_rate as f64)  ) * ( 1000000000.0 ); 
    //                        ^^^^ - left and right both have samp_rate samples per sec
    println!( " period/sample : {:?}(ns)(mono,alternated)", ns_per_sample);
    let ns_per_bit:f64 =  (0.50 / ((typspec.bits_per_sample as f64) * (typspec.sample_rate as f64))  ) * ( 1000000000.0 ); 
    println!( " period/bit : {:?}(ns) ( wrong, ideal, too long)", ns_per_bit);
    let ns_per_timeslot:f64 =  (0.50 / ((32 as f64) * (typspec.sample_rate as f64))  ) * ( 1000000000.0 ); 
    let ns_per_hemitimeslot:f64 =  (0.25 / ((32 as f64) * (typspec.sample_rate as f64))  ) * ( 1000000000.0 ); 
    println!( " period/timeslot : {:?}/ {:?}(ns) ( aes 32 bit)", ns_per_timeslot, ns_per_hemitimeslot);

    let mut which_preamble = 2;

    for i in 0..odo.len () { // loop through samples  strided left (even )  and right (odd)
        let mut tstamp:u64; 
        tstamp = ((ns_per_sample * (i as f64))  as u64);

        writr.timestamp ( tstamp ); // XXfixme for ns timstamp vs samp_rate
        writr.change_scalar (lrclock,  ( i%2 ==0)   ).expect("verr");
	writr.change_scalar (noamiout, false) .expect("aessetpreambleerr");

	if i%192 == 0 { which_preamble = 2 ;} // z preambles every 192 frames

	for hemitimeslot in 0..7 {
		let the_time = (tstamp as f64) +( ( hemitimeslot as f64 )  * ns_per_hemitimeslot );
        	writr.timestamp ( the_time as u64 ).expect ("ts"); // XXfixme for ns timstamp vs samp_rate
            	writr.change_scalar (aesout, preambles[which_preamble][hemitimeslot]  ).expect("aessetpreambleerr");
	}

	if ( which_preamble == 2) { 
		which_preamble = 0;
	} else if ( which_preamble == 1 ) { 
		which_preamble =0;	
	} else { which_preamble +=1;}


	let mut amiflip = false;
	for timeslot in 4..31 {
		let the_time = (tstamp as f64) +( (timeslot as f64 )  * ns_per_timeslot );
		let mut payload = false;
		if ( timeslot <=27 ) {
			let shifted = odo[i].checked_shr( timeslot - 4);
			if shifted == None {
				payload = false;
			} else {
				payload= shifted.unwrap() & 1 == 1;
			}
		}
		if ( timeslot == 28 ){ payload = true;} //valid
		if ( timeslot == 29 ){ payload = false;} //user data
		if ( timeslot == 30 ){ payload = true;} //status
		if ( timeslot == 31 ) {payload = (odo[i]%2 == 1 );} // parity XXXX

        	writr.timestamp ( the_time as u64 ).expect ("ts"); 
		
		if ( payload == false ) {
			writr.change_scalar (aesout, amiflip ) .expect("aessetpreambleerr");
		} else {
			writr.change_scalar (aesout, amiflip ) .expect("aessetpreambleerr");
			amiflip ^= true;
			let half_bit_time = the_time + ( (ns_per_timeslot / 2.0) as f64);
        		writr.timestamp ( half_bit_time as u64 ).expect ("ts"); 
			writr.change_scalar (aesout, amiflip ) .expect("aessetpreambleerr");
			
		}
        	writr.timestamp ( the_time as u64 ).expect ("ts"); 
            	writr.change_scalar (noamiout, payload) .expect("aessetpreambleerr");
		amiflip ^= true
	}
        
  
    } 
}
