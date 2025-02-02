extern crate core;

mod comment_ranking;

use crate::comment_ranking::comment_ranking::SortScoringMethod;
use comment_ranking::comment_ranking::{CommentListSortingMethod, CommentScoringMethod};
use std::fs::File;
use std::thread;


fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("Running comment simulations, world!");
	let comment_scoring_method = CommentScoringMethod::RawScore;
	let comment_file_header = format!("commentRanking-{}-", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
	for score_sorting_method in [SortScoringMethod::Linear] {//, SortScoringMethod::PlaceSquared
		for sorting_method in [CommentListSortingMethod::Best] {//, CommentListSortingMethod::Top  CommentListSortingMethod::New
			let filename = format!("{}-{}-{}.csv", comment_file_header, sorting_method, score_sorting_method);
			let file = File::create(&filename).expect("Failed to create file");
			let mut wtr = csv::Writer::from_writer(file);

			wtr.write_record(&["number of comments", "number of user interactions", "ranking result"])?;
			let comment_number_step_size = 5;
			let user_interaction_step_size = 100;
			let maximum_number_of_comments = 20;
			let maximum_number_of_user_interactions = 100;
			for number_of_comments in (comment_number_step_size..maximum_number_of_comments).step_by(comment_number_step_size.into()) {
				for number_user_interactions in (number_of_comments..maximum_number_of_user_interactions).step_by(user_interaction_step_size) {
					let mut children = vec![];
					let number_of_runs = 5;
					for index_of_runs in 0..number_of_runs {
						children.push(thread::spawn(move || -> (usize, f32) {
							let ranking_result = comment_ranking::comment_ranking::simulate_comments_for_one_topic(number_of_comments,
																												   number_user_interactions,
																												   comment_scoring_method,
																												   sorting_method,score_sorting_method
							);
//							println!("child rank res {},{}",index_of_runs, ranking_result);
							(index_of_runs, ranking_result)
						}));
					}
					let mut ranking_average = 0.0;
					let mut ranking_events_to_average = 0;
					for child in children {
						let (run_index, ranking_result) = child.join().unwrap();
						ranking_events_to_average += 1;
						ranking_average += ranking_result;
					}
//					ranking_average = ranking_average / (number_of_runs as f32);
					let one_record = [
						//					sorting_method.to_string(),
						number_of_comments.to_string(),
						number_user_interactions.to_string(),
						(ranking_average/(ranking_events_to_average as f32)).to_string(),
						//					number_of_runs.to_string(),
					];
					wtr.write_record(&one_record)?;
				}
				println!("{}", number_of_comments);
			}
			wtr.flush()?;
		}
	}
	Ok(())
}
