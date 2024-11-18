extern crate core;

mod comment_ranking;

use comment_ranking::comment_ranking::{CommentListSortingMethod, CommentScoringMethod};
use std::fs::File;
use std::thread;


fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("Hello, world!");
	let comment_file_header = format!("commentRanking-{}-", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());

	for sorting_method in [CommentListSortingMethod::Best, CommentListSortingMethod::Controversial, CommentListSortingMethod::Hot, CommentListSortingMethod::New, CommentListSortingMethod::Top] {
		let filename = format!("{}{}.csv",comment_file_header, sorting_method);
		let file = File::create(&filename).expect("Failed to create file");
		let mut wtr = csv::Writer::from_writer(file);

		wtr.write_record(&["number of comments", "number of user interactions", "ranking result"])?;
		for number_of_comments in 3..500 {
			for number_user_interactions in number_of_comments..1000 {
				let mut children = vec![];
				let number_of_runs = 10;
				for index_of_runs in 0..number_of_runs {
					children.push(thread::spawn(move || -> (usize, f32) {
						let ranking_result = comment_ranking::comment_ranking::simulate_comments_for_one_topic(
							number_of_comments,
							number_user_interactions,
							CommentScoringMethod::ThumbsUpDown,
							sorting_method,
						);
						(index_of_runs, ranking_result)
					}));
				}
				let mut ranking_average = 0.0;
				for child in children {
					let (run_index,ranking_result) = child.join().unwrap();
					ranking_average += ranking_result;
				}
				ranking_average = ranking_average / number_of_runs as f32;
				let one_record = [
//					sorting_method.to_string(),
					number_of_comments.to_string(),
					number_user_interactions.to_string(),
					ranking_average.to_string(),
//					number_of_runs.to_string(),
				];
				wtr.write_record(&one_record)?;
			}
			println!("{}", number_of_comments);
		}
		wtr.flush()?;
	}
	Ok(())
}