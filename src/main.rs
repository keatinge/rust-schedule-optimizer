use std::fmt::{self,Debug};
use std::io::{Write, Read};
use std::collections::{HashSet, HashMap};
use std::fs::File;
use std::cmp::Ordering::{Less, Equal, Greater};
extern crate select;
#[macro_use] extern crate serde_json;
use select::predicate::{Predicate, Attr, Class as HTMLClass, Name, And};


const NBSP:char = '\u{a0}';

const CRN_INDEX:usize = 1_usize;
const DEPT_INDEX:usize = 2_usize;
const COURSE_IDNEX:usize = 3_usize;
const SEC_IDNEX:usize = 4_usize;
const CRED_INDEX:usize = 6_usize;
const TITLE_INDEX:usize = 7_usize;
const DAYS_INDEX:usize = 8_usize;
const TIME_INDEX:usize = 9_usize;
const INSTRUCTOR_INDEX:usize = 19_usize;
const LOC_INDEX:usize = 21_usize;






struct TimeDuration {
    day: u8, // Todo vec<day?> // todo just precompute all section collisions? // howmany are tehre?
    hour: u32,
    minutes: u32,
    length_in_minutes: u32,
}

impl Debug for TimeDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} at {} for {} mins", u8_to_day_char(self.day), self.hour, self.length_in_minutes)
    }
}

impl TimeDuration {

    fn minute_begin(&self) -> u32 {
        let minutes_per_day = 24 * 60;
        let mb = self.hour * 60 + self.minutes;
        assert!(mb < minutes_per_day);
        return mb;
    }

    fn minute_end(&self) -> u32 {
        self.minute_begin() + self.length_in_minutes
    }
    fn intersects_with(&self, other: &Self) -> bool {
        if self.day != other.day {
            return false;
        }

        let this_begin = self.minute_begin();
        let this_end = self.minute_end();

        let other_begin = other.minute_begin();
        let other_end = other.minute_end();

        // If one ends before the other begins they don't overlap
        if this_end < other_begin || other_end < this_begin {
            return false;
        }

        // One does not end before the other begins, which implies overlap
        return true
    }

    fn to_json(&self) -> serde_json::Value {
        assert!(self.minutes == 0);
        json!({
            "day" : self.day+1,
            "hour" : self.hour,
            "length" : self.length_in_minutes
        })
    }
}


#[derive(Debug)]

struct Section<'a> {
    name: &'a str,
    dept: &'a str,
    course_num: u32,
    section_num: u8,
    credits: u8,
    times: Vec<TimeDuration>
}

impl<'a> Section<'a> {
    fn intersects_with(&self, other: &Self) -> bool {
        //Todo: this can be done faster with sorting

        for time1 in self.times.iter() {
            for time2 in other.times.iter() {
                if time1.intersects_with(time2) {
                    return true;
                }
            }
        }
        return false;

    }


    fn to_json(&self) -> serde_json::Value {
        json!({
            "name" : format!("{}-{}-{}", self.dept, self.name, self.section_num),
            "times" : self.times.iter().map(TimeDuration::to_json).collect::<Vec<_>>()
        })
    }
}

struct Class<'a> {
    name: &'a str,
    sections: Vec<Section<'a>>
}


#[derive(Debug)]
struct Course<'a> {
    name: &'a str,
    department: &'a str,
    sections: Vec<Section<'a>>
}



//fn make_single_section_hour<'a>(n:&'a str, course_num:u32, start_hour: u32) -> Class<'a> {
//
//    let only_time = TimeDuration {day: 0, hour: start_hour, minutes: 0, length_in_minutes: 50};
//    let section = Section {course_num, section_num: 1, credits: 4, times: vec![only_time], name: n};
//
//    Class {name: n, sections: vec![section]}
//
//}


fn print_stack(stack: &[&Section]) {
    for (i, sec) in stack.iter().enumerate() {
        print!("{}-{}", sec.name, sec.section_num);

        if i != stack.len() - 1 {
            print!(" -> ");
        }
    }


}

fn intersects_any_in_stack(sec_to_check: &Section, stack: &[&Section]) -> bool {
    for sec in stack.iter() {
        if sec_to_check.intersects_with(sec) {

            return true;
        }
    }
    return false;
}


fn schedule_credit_hours(sched: &Vec<&Section>) -> u32 {
    sched.iter().map(|s| s.credits as u32 ).sum::<u32>()
}

fn schedule_starting_time(sched: &Vec<&Section>) -> u32 {
    sched.iter().map(|section| {
        section.times.iter().min_by_key(|t| t.hour).expect("Found a section with no times.... wtf?").hour
    }).min().unwrap()
}


#[derive(Debug)]
struct TimeAnalysis {
    back_to_dorm_count: u32,
    back_to_dorm_minutes: u32,
    bs_time: u32
}


fn schedule_bs_time(sched: &Vec<&Section>) -> TimeAnalysis {
    // This can be optimized by preallocing a big matrix of max size 7*24 but can be 7*(less_than_24)
    // and avoiding any sorting

    let mut day_partition:Vec<_> = (0..7).map(|_| Vec::<&TimeDuration>::new()).collect();

    for &sec in sched.iter() {
        for time_dur in sec.times.iter() {
            let day = time_dur.day;
            assert!(day >= 0 && day < 7);

            day_partition[day as usize].push(&time_dur);
        }
    }


    let mut bs_time = 0_u32;
    let mut back_to_dorm_count = 0_u32;
    let mut total_btd_break_minutes = 0_u32;


    for dp in day_partition.iter_mut() {
        dp.sort_by_key(|td| td.hour);
        for win in dp.as_slice().windows(2) {
            let first_end = win[0].minute_end();
            let second_begin = win[1].minute_begin();

            let dm = second_begin - first_end;
            let two_hours = 2 * 60;


            if dm < two_hours {
                if dm != 10 {
                    bs_time += dm;
                }
            } else {
                total_btd_break_minutes += dm;
                back_to_dorm_count += 1
            }
            assert!(dm > 0);

        }
    }

    TimeAnalysis {
        back_to_dorm_count,
        back_to_dorm_minutes : total_btd_break_minutes,
        bs_time
    }

}

#[derive(Debug, Clone)]
struct EvaluatedSchedule<'a> {
    schedule: Vec<&'a Section<'a>>,
    s_time: u32,
    start_time_dt: u32,
    bs_time: u32,
    back_to_dorm_count: u32,
    back_to_dorm_minutes: u32,
    score: f64
}

impl<'a> EvaluatedSchedule<'a> {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "score" : self.score,
            "stats" : {
                "s_time" : self.s_time,
                "start_time_dt" : self.start_time_dt,
                "bs_time" : self.bs_time,
                "back_dorm_count" : self.back_to_dorm_count,
                "back_to_dorm_minutes" : self.back_to_dorm_minutes,
            },
            "classes" : self.schedule.iter().map(|s| s.to_json()).collect::<Vec<_>>()
        })
    }


    fn is_strictly_better_than(&self, other:&Self) -> bool {
        let cmps = vec![
            self.start_time_dt.cmp(&other.start_time_dt),
            self.bs_time.cmp(&other.bs_time),
            self.back_to_dorm_count.cmp(&other.back_to_dorm_count),
            self.back_to_dorm_minutes.cmp(&other.back_to_dorm_minutes),
        ];

        let better = vec![
            Less,
            Less,
            Less,
            Less
        ];


        assert_eq!(better.len(), cmps.len());
        let opposite = |o: &std::cmp::Ordering| {
            match o {
                &Less => Greater,
                &Greater => Less,
                _=> panic!()
            }
        };

        // Make sure it isn't any worse
        for (cmp, desired_cmp) in cmps.iter().zip(better.iter()) {
            let opp = opposite(desired_cmp);

            if *cmp == opp {
                return false;
            }
        }


        // Make sure atleast one is better

        for (cmp, desired_cmp) in cmps.iter().zip(better.iter()) {
            if cmp == desired_cmp {
                return true;
            }
        }

        return false; // They are equal




    }

}

fn evaluate_schedule<'a>(sched: Vec<&'a Section>) -> EvaluatedSchedule<'a> {
    let credit_hours = schedule_credit_hours(&sched);

    // Todo I don't really care about absolute earliest, maybe try average or drop the single worst
    let s_time = schedule_starting_time(&sched);
    let time_analysis = schedule_bs_time(&sched);
    //print_stack(sched);




    let dt:u32 = (s_time as i32 - 12).abs() as u32;
    let bs_time_penalty = time_analysis.bs_time as f64;
    let back_to_dorm_count_penalty = time_analysis.back_to_dorm_count as f64 * 20.0;
    let back_to_dorm_minutes_penalty = time_analysis.back_to_dorm_minutes as f64 / 100.0;


    let mut total_score = 0.0 - (dt as f64) - bs_time_penalty - back_to_dorm_count_penalty - back_to_dorm_minutes_penalty;


    //TODO
    if credit_hours < 16 || credit_hours > 20 {
        total_score = -std::f64::INFINITY;
    }
    //print!(" credit_hours = {:?}, s_time = {:?}, bs_time={:?}, -- SCORE= {:?}", credit_hours, s_time, time_analysis, total_score);
    //print!("\n");
    EvaluatedSchedule {
        schedule: sched,
        score: total_score,
        start_time_dt: dt,
        bs_time: time_analysis.bs_time,
        back_to_dorm_count: time_analysis.back_to_dorm_count,
        back_to_dorm_minutes: time_analysis.back_to_dorm_minutes,
        s_time: s_time

    }
}



fn enumerate_schedules<'b, 'c, 'a : 'b+'c, >(courses: &'a[Course<'a>], stack: &'b mut Vec<&'a Section<'a>>, results: &'c mut Vec<Vec<&'a Section<'a>>>) {
    if courses.len() == 0 {
        if stack.len() != 0 && stack.len() == 4{

            results.push(stack.clone());
        }
        return
    }


    let this_course = &courses[0];

    // You can either not take this course at all
    enumerate_schedules(&courses[1..], stack, results);

    // Or you can take this course at any of the below times
    for section in this_course.sections.iter() {

        if !intersects_any_in_stack(section, stack) {
            stack.push(section);
            enumerate_schedules(&courses[1..], stack, results);
            stack.pop();
        }

    }
}


fn get_immediate_inner_text<'a>(n: select::node::Node<'a>) -> Option<&'a str> {
    n.children().next().and_then(|c| {
        c.as_text().and_then(|t| {
            if t.len() == 2 && t.chars().nth(0).unwrap() == NBSP {
                None
            }
            else {
                Some(t)
            }
        })
    })

}


// Takes a string like 06:00 pm and returns time in 24 hour format
fn parse_mins_hours(s:&str) -> (u32, u32) {

    let mut time_and_pm_am = s.split(' ');

    let time = time_and_pm_am.next().unwrap();
    let am_pm = time_and_pm_am.next().unwrap();


    let mut pieces = time.split(':');

    let mut hours:u32 = pieces.next().unwrap().parse().unwrap();
    let mins:u32 = pieces.next().unwrap().parse().unwrap();


    if am_pm == "pm" && hours != 12 {
        hours = hours + 12;

    }else if am_pm == "am" && hours == 12 {
        hours = 0;
    }


    (hours, mins)
}

fn day_char_to_u8(day:char) -> u8 {
    let days = ['M', 'T', 'W', 'R', 'F'];
    days.iter().position(|&c| c == day).unwrap() as u8
}
fn u8_to_day_char(day:u8) -> char {
    let days = ['M', 'T', 'W', 'R', 'F'];
    return days[day as usize]
}

fn parse_time(days:&str, s:&str) -> Vec<TimeDuration> {
    let mut durs = Vec::new();
    for ch in days.chars() {
        let mut pieces = s.split('-');

        let start = pieces.next().unwrap();
        let end = pieces.next().unwrap();

        let (s_h, s_m) = parse_mins_hours(start);
        let (e_h, e_m) = parse_mins_hours(end);

        let dh = e_h - s_h;
        let dm = (60 - s_m) % 60 + e_m;

        let total_dm = dh * 60 + dm;


        let this_td = TimeDuration{
            day: day_char_to_u8(ch),
            hour: s_h,
            minutes: s_m,
            length_in_minutes: total_dm
        };
        durs.push(this_td)

    }
    durs
}


fn get_time_in_row<'a, 'b>(rows:&'b Vec<select::node::Node<'a>>, row_i:usize) -> Option<Vec<TimeDuration>> {
    let next_row = rows[row_i];
    let next_row_tds =  next_row.find(Name("td")).collect::<Vec<_>>();

    if next_row_tds.len() == 0 {
        return None;
    }


    let next_time = get_immediate_inner_text(next_row_tds[TIME_INDEX]);
    let next_days = get_immediate_inner_text(next_row_tds[DAYS_INDEX]);

    if next_time.is_some() && next_time.unwrap() != "TBA" && next_days.is_some() {
       Some(parse_time(next_days.unwrap(), next_time.unwrap()))
    } else {
        None
    }
}


fn get_time_continuation<'a, 'b>(rows:&'b Vec<select::node::Node<'a>>, row_i:usize) -> Option<Vec<TimeDuration>> {


    if row_i >= rows.len() {
        return None
    }
    get_time_in_row(rows, row_i).and_then(|v:Vec<TimeDuration>| {

        let next_row = rows[row_i];
        let next_row_tds =  next_row.find(Name("td")).collect::<Vec<_>>();
        if next_row_tds.len() == 0 {
            return None;
        }
        if get_immediate_inner_text(next_row_tds[TITLE_INDEX]).is_some() {
            return None
        }

        Some(v)


    })


}


fn load_sections_from_doc<'a>(doc:&'a select::document::Document) ->  HashMap<(&'a str, u32), Course<'a>> {


    let table = doc.find(And(Name("table"), HTMLClass("datadisplaytable"))).next().expect("Couldn't find <table class=\"datadisplaytable\">");
    let rows = table.find(Name("tr")).collect::<Vec<_>>();
    println!("Found rows");

    // Todo my layout of times can be optimized, it appears every section only has 1 time but multiple days

    let mut courses = HashMap::<(&'a str, u32), Course<'a>>::new();



    let mut i = 0_usize;
    while i < rows.len() {
        let row = &rows[i];
        let tds = row.find(Name("td")).collect::<Vec<_>>();
        if tds.len() == 0 {
            println!("Found header row {}", row.inner_html());
            i += 1;
            continue; // This is a header row
        }

        let title_txt = get_immediate_inner_text(tds[TITLE_INDEX]);

        println!("title_text={:?}", title_txt);

        let course_num_s = get_immediate_inner_text(tds[COURSE_IDNEX]);
        let course_num_i:Result<u32, _> = course_num_s.unwrap().parse();

        let section_s = get_immediate_inner_text(tds[SEC_IDNEX]);
        let section_num_i:Result<u8, _> = section_s.unwrap().chars().filter(|c| *c != 'T' && *c != 'G').collect::<String>().parse();

        let credits_s = get_immediate_inner_text(tds[CRED_INDEX]);


        let dept_s = get_immediate_inner_text(tds[DEPT_INDEX]);

        // Todo double check this and make sure there aren't any non_integer credits
        let credit_num_i:Result<u8, _> = credits_s.unwrap().chars().take_while(|&c| c!= '.').collect::<String>().parse();

        if course_num_i.is_err() {
            panic!("Couldn't parse {:?}", course_num_s);
        }

        let td_opt = get_time_in_row(&rows, i);
        if td_opt.is_none() {
            println!("SKIPPING ROW {:?} at i={} BECAUSE FOUND NO TIME", title_txt, i);
            i+= 1;
            continue;
        }

        let mut td_vec = td_opt.unwrap();

        while let Some(new_td) = get_time_continuation(&rows, i+1) {
            println!("GOTTT A CONTINUATION!!!");
            td_vec.extend(new_td);
            i += 1
        }


        let course_num = course_num_i.unwrap();
        let course_name = title_txt.unwrap();
        let dept_str = dept_s.unwrap();



        let sec = Section {
            name : course_name,
            course_num: course_num,
            section_num: section_num_i.unwrap(),
            credits: credit_num_i.unwrap(),
            times: td_vec,
            dept: dept_str,

        };

        let existing_course:&mut Course<'a> = courses.entry((dept_str, course_num)).or_insert(Course { department: dept_str, name:course_name, sections: Vec::new() });
        existing_course.sections.push(sec);


        i += 1;

    }

    //println!("{:#?}", courses);
    return courses;
}



fn prune<'a>(v:&mut Vec<EvaluatedSchedule<'a>>) -> Vec<EvaluatedSchedule<'a>> {

    // Todo this is much faster with a hashset

    let mut strictly_worse_i = HashSet::with_capacity(5000); //todo


    for i in 0..v.len() {
        for j in 0..v.len() {
            if i==j || strictly_worse_i.contains(&i) || strictly_worse_i.contains(&j) {
                continue;
            }

            if v[i].is_strictly_better_than(&v[j]) {
                strictly_worse_i.insert(j);
            } else if v[j].is_strictly_better_than(&v[i]) {
                strictly_worse_i.insert(i);
            }
        }
    }



    let mut ret = Vec::new();

    for i in 0..v.len() {
        if strictly_worse_i.contains(&i) { continue; }
        ret.push(v[i].clone()); // todo should i clone here?
    }


    println!("Successfuly pruned {} schedules or {}%!!", strictly_worse_i.len(), strictly_worse_i.len() as f64 / v.len() as f64 * 100.);


    return ret;
}



fn main() {

    let mut file = std::fs::File::open("./Search Results.html").expect("Couldn't open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Couldn't read file");

    let doc = select::document::Document::from(contents.as_ref());


    let mut courses = load_sections_from_doc(&doc);


    let c1 = courses.remove(&("MATH", 1010));
    let c2 = courses.remove(&("PHYS", 1100));
    let c3 = courses.remove(&("CSCI", 1200));
    let c4 = courses.remove(&("PSYC", 1200));


    let courses = [c1.unwrap(), c2.unwrap(), c3.unwrap(), c4.unwrap()];
    let mut stack = Vec::new();
    let mut results = Vec::new();

    enumerate_schedules(&courses, &mut stack, &mut results);

    println!("Generated {:?} schedules", results.len());

    let mut evaluations = results.into_iter().map(|s| evaluate_schedule(s)).collect::<Vec<_>>();
    evaluations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());



    let SHOULD_PRUNE = true;
    let evals_for_json = if SHOULD_PRUNE {
        prune(&mut evaluations)
    } else {
        evaluations
    };



    let schedules = json!({
        "schedules" : evals_for_json.iter().map(EvaluatedSchedule::to_json).collect::<Vec<_>>()
    });

    let full_data = format!("let schedules = JSON.parse('{}')", schedules.to_string());
    let mut file = File::create("data2.js").unwrap();
    file.write_all(full_data.as_bytes());


}



#[test]
fn time_durations() {
    let t1 = TimeDuration {day: 0, hour: 1, minutes: 30, length_in_minutes: 60}; // Hour class starting at 1:30 to 2:30
    let t2 = TimeDuration {day: 0, hour: 1, minutes: 30, length_in_minutes: 60};


    assert!(t1.intersects_with(&t2));
    assert!(t2.intersects_with(&t1));

    let tt1 = TimeDuration {day: 0, hour: 1, minutes: 30, length_in_minutes: 60}; // 1:30 - 2:30
    let tt2 = TimeDuration {day: 0, hour: 2, minutes: 29, length_in_minutes: 60}; // 2:29 - 3:29
    let tt3 = TimeDuration {day: 0, hour: 2, minutes: 30, length_in_minutes: 60}; // 2:30 - 3:30

    assert!(tt1.intersects_with(&tt2));
    assert!(tt2.intersects_with(&tt1));
    assert!(tt3.intersects_with(&tt1));
}


#[test]
fn time_parsing() {
    assert_eq!(parse_mins_hours("6:00 pm"), (6+12, 0));
    assert_eq!(parse_mins_hours("12:00 am"), (0, 0));
    assert_eq!(parse_mins_hours("12:00 pm"), (12, 0));
    assert_eq!(parse_mins_hours("7:15 am"), (7, 15));
    assert_eq!(parse_mins_hours("8:35 pm"), (8+12, 35))
}

#[test]
fn parse_time_test() {
    unimplemented!();
}