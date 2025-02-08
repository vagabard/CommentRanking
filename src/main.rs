extern crate core;

mod comment_ranking;

use crate::comment_ranking::comment_ranking::{LowScoreMemberHandling, SortScoringMethod};
use comment_ranking::comment_ranking::{CommentListSortingMethod, CommentScoringMethod};
use std::fs::File;
use std::thread;
use std::env;


fn main() -> Result<(), Box<dyn std::error::Error>> {
	env::set_var("RUST_BACKTRACE", "");
	println!("Running comment simulations, world!");
	let comment_scoring_method = CommentScoringMethod::RawScore;
	let comment_file_header = format!("commentRanking-{}-", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
	for score_sorting_method in [SortScoringMethod::NormalizedLinear, SortScoringMethod::NormalizedPlaceSquared] {//SortScoringMethod::Linear,SortScoringMethod::PlaceSquared, 
		for sorting_method in [CommentListSortingMethod::Best] {//, CommentListSortingMethod::Top  CommentListSortingMethod::New
			for handle_low_score_members in [LowScoreMemberHandling::Ignore , LowScoreMemberHandling::Proportional_To_Score_Chance, LowScoreMemberHandling::Flat_Percent_Chance] {
				let filename = format!("{}-{}-{}-{}-{}.csv", comment_file_header, comment_scoring_method, sorting_method, score_sorting_method, handle_low_score_members);
				let file = File::create(&filename).expect("Failed to create file");
				let mut wtr = csv::Writer::from_writer(file);

				wtr.write_record(&["number of comments", "number of user interactions", "ranking result"])?;
				let comment_number_step_size =50;
				let user_interaction_step_size = 100;
				let maximum_number_of_comments = 501;
				let maximum_number_of_user_interactions = 10000;
				for number_of_comments in (comment_number_step_size..maximum_number_of_comments).step_by(comment_number_step_size) {
					//				for number_user_interactions in (number_of_comments..maximum_number_of_user_interactions).step_by(user_interaction_step_size) {
//					let maximum_number_of_user_interactions = number_of_comments as u32 * 20;
//					let user_interaction_step_size = maximum_number_of_user_interactions / 10;
					let mut children = vec![];
					let number_of_runs = 10;
					for index_of_runs in 0..number_of_runs {
						children.push(thread::spawn(move || -> (usize,Vec<(u32, f32)>) {
							let ranking_result = comment_ranking::comment_ranking::simulate_comments_for_one_topic(number_of_comments as u32,
																												   maximum_number_of_user_interactions,
																												   comment_scoring_method,
																												   sorting_method,score_sorting_method, user_interaction_step_size ,handle_low_score_members 
							);
							//							println!("child rank res {},{}",index_of_runs, ranking_result);
							(index_of_runs, ranking_result)
						}));
					}
					let mut ranking_average = vec![(0,0.0);(maximum_number_of_user_interactions/user_interaction_step_size) as usize];
					let mut ranking_events_to_average = 0;
					for child in children {
						let (_run_index, ranking_result) = child.join().unwrap();
						for index in 0..ranking_result.len() {
							//							println!("run index {} index {} {} Aaver len {}",_run_index, index, ranking_result.len(), ranking_average.len());
							ranking_average[index].0 = ranking_result[index].0;// this only needs to be done once, but this is cheaper than  checking if it's been done.
							ranking_average[index].1 += ranking_result[index].1;
						}
						ranking_events_to_average += 1;
					}
					//					ranking_average = ranking_average / (number_of_runs as f32);
					for index in 0..ranking_average.len() {

						let one_record = [
							//					sorting_method.to_string(),
							number_of_comments.to_string(),
							ranking_average[index].0.to_string(),
							(ranking_average[index].1 / (ranking_events_to_average as f32)).to_string()
							//					number_of_runs.to_string(),
						];
						wtr.write_record(&one_record)?;
					}
					//				}
					println!("{}", number_of_comments);
				}
				wtr.flush()?;
			}
		}
	}
	Ok(())
}
