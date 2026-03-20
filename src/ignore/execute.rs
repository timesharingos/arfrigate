use crate::ignore::repo::FilterList;

pub fn run_filter(target: Vec<String>) {
    let multilist = FilterList::new(target);
    let final_result = multilist.get_actual().join("\n");
    print!("{}", final_result);
}
