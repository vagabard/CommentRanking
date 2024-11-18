pub mod comment_ranking {
	use strum_macros::Display;
	use rand::{thread_rng, Rng};
	use rand_distr::{Distribution, Normal};
	use std::cmp::Ordering;
	use rand::seq::SliceRandom;
	use rand_distr::num_traits::pow;
	use rand_distr::Beta;

	use strum_macros::EnumIter;

	use log::info;

	const AVERAGE_NUMBER_OF_COMMENTS_VIEWED_BY_USER: f64 = 3.0;

	#[derive(Clone, Copy)]
	pub enum SortScoringMethod {
		Linear,
		PlaceSquared,
	}
	#[derive(Clone, Copy)]
	pub enum CommentScoringMethod {
		ThumbsUpDown,
		ZeroToTen,
	}

	#[derive(Clone, Copy, Debug, EnumIter,Display )]
	pub enum CommentListSortingMethod {
		Top,
		New,
		Hot,
		Best,
		Controversial,
	}


	#[derive(Copy, Clone, Debug)]
	struct User {
		id: u16,
		reputation: f32,
		scoring_accuracy: f32,
		preferred_sorting: CommentListSortingMethod,
		//user_comments: Vec<Comment>
	}

	#[derive(Clone, Copy, Debug)]
	struct AssignedScore {
		score: f32,
		time: u16,
	}

	#[derive(Clone, Debug)]
	struct Comment<'a> {
		true_quality: f32,
		perceived_quality: f32,
		user_scores: Vec<AssignedScore>,// tuple of the score given and when given
		creator: &'a User,
		creation_time: u16,
	}

	impl<'a> Comment<'a> {
		fn add_assigned_score(&mut self, score: AssignedScore) {
			self.user_scores.push(score);
		}
	}

	impl Eq for Comment<'_> {}
	impl PartialEq<Self> for Comment<'_> {
		fn eq(&self, other: &Self) -> bool {
			self.true_quality.eq(&other.true_quality)
		}
	}
	impl PartialOrd<Self> for Comment<'_> {
		fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
			self.true_quality.partial_cmp(&other.true_quality)
		}
	}
	impl Ord for Comment<'_> {
		fn cmp(&self, other: &Self) -> Ordering {
			self.true_quality.partial_cmp(&other.true_quality).unwrap()
		}
	}


	pub fn simulate_comments_for_one_topic(number_of_comments: u16, number_of_user_interactions: u16, comment_scoring_method: CommentScoringMethod, comment_list_sorting_type: CommentListSortingMethod) -> f32 {
//		env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
		let mut rng = thread_rng();
		let users = generate_user_list(number_of_user_interactions);
		let mut comment_list_result: Vec<Comment> = vec![];
		for number_of_views_for_topic in 0..number_of_user_interactions {
			let one_user = users.choose(&mut rng).unwrap();
			let chance_to_make_comment: f64 = rng.gen();
			let make_comment_threshold = number_of_comments as f64 / number_of_user_interactions as f64;
			let should_make_comment = make_comment_threshold > chance_to_make_comment;
			let space_for_comment_available = comment_list_result.len() < number_of_comments as usize;
			let comment_required = should_make_comment || comment_list_result.len() < 1;

			if space_for_comment_available && comment_required {
				//add comment
				let comment = generate_one_comments(&one_user, number_of_views_for_topic);
				comment_list_result.push(comment);
			}
			simulate_one_user_interaction(&mut comment_list_result, one_user, number_of_views_for_topic, comment_scoring_method)
		}
		let mut binding = comment_list_result.to_vec();
		let mut position_score_comment_list_optimal = calculate_sorting_scores_for_list(&mut binding, comment_list_sorting_type, true);
		info!("pre sorted list {:?}", scored_comment_list_to_string(&position_score_comment_list_optimal));
		position_score_comment_list_optimal.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
		info!("ppost sorted list {:?}", scored_comment_list_to_string(&position_score_comment_list_optimal));

		let optimally_sorted_list: Vec<Comment> = position_score_comment_list_optimal.into_iter().map(|(_, comment)| comment.clone()).collect();
		let maximum_possible_score = calculate_sorted_comment_list_score(&optimally_sorted_list, SortScoringMethod::Linear);
		info!("ppost sorted list {:?}", comment_list_to_string(&optimally_sorted_list));

		let mut binding = comment_list_result.to_vec();
		let mut position_score_comment_list_perceived = calculate_sorting_scores_for_list(&mut binding, comment_list_sorting_type, false);
		position_score_comment_list_perceived.sort_by(|a, b| {a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)});
		let user_scored_sorted_list: Vec<Comment> = position_score_comment_list_perceived.into_iter().map(|(_, comment)| comment.clone()).collect();
		let user_score = calculate_sorted_comment_list_score(&user_scored_sorted_list, SortScoringMethod::Linear);
		info!("max score {} user_score {} ", maximum_possible_score,user_score);

		user_score / maximum_possible_score
	}

	fn scored_comment_list_to_string(comments : &Vec<(f32,&Comment)>) -> String {
		let mut comment_list_string = String::new();
		for comment in comments {
			comment_list_string.push_str(&format!("score {} ",comment.0)); // t {} p {} ",comment.0, comment.1.true_quality, comment.1.perceived_quality));
		}
		comment_list_string
	}
	fn comment_list_to_string(comments : &Vec<Comment>) -> String {
		let mut comment_list_string = String::new();
		for comment in comments {
			comment_list_string.push_str(&format!(" {}",comment.true_quality)); // t {} p {} ",comment.0, comment.1.true_quality, comment.1.perceived_quality));
		}
		comment_list_string
	}

	fn simulate_one_user_interaction(list_of_comments: &mut Vec<Comment>, user: &User, time_of_viewing: u16, comment_scoring_method: CommentScoringMethod) -> () {
		let mut number_of_comments_to_view = thread_rng().sample(Normal::new(AVERAGE_NUMBER_OF_COMMENTS_VIEWED_BY_USER, 1.).unwrap()) as u16;
		let beta = Beta::new(5.0, 5.0).unwrap();
		number_of_comments_to_view += 1; // make sure at least one comment is viewed, assume every user reads at least the top comment
		if number_of_comments_to_view > list_of_comments.len() as u16 {
			number_of_comments_to_view = list_of_comments.len() as u16;
		}
		for comment_index in 0..number_of_comments_to_view {
			//if list_of_comments.len() >= (comment_index + 1) as usize {}
			let one_comment = list_of_comments.get_mut(comment_index as usize).unwrap();
			let scoring_noise = beta.sample(&mut thread_rng()) + 0.5;// beta goes from 0 to 1, we want this centered around 1
			let new_comment_score = scoring_noise * user.scoring_accuracy * one_comment.true_quality; // multiply how accurate the user is at true quality, and quality.  Then add some noise
			let converted_score = convert_user_comment_score_to_comment_scoring_system(new_comment_score, comment_scoring_method);
			one_comment.add_assigned_score(AssignedScore { score: converted_score, time: time_of_viewing })
		}
	}

	fn generate_one_comments(user: &User, time: u16) -> Comment {
		let mut rng = thread_rng();

		// use a beta distribution rather than normal so all scores are between 0 and 1
		let beta = Beta::new(5.0, 5.0).unwrap();
		let comment_score = beta.sample(&mut rng) as f32;
		let one_comment = Comment { true_quality: comment_score, perceived_quality: 0.0, user_scores: Vec::new(), creator: user, creation_time: time };
		one_comment
	}

	fn generate_user_list(number_of_users_to_create: u16) -> Vec<User> {
		let mut users = Vec::new();
		let mut rng = thread_rng();
		let beta = Beta::new(5.0, 5.0).unwrap();
		for user_index in 0..number_of_users_to_create {
			let user_scoring_accuracy = beta.sample(&mut rng) + 0.5;
			let one_user = User { id: user_index, reputation: beta.sample(&mut rng), scoring_accuracy: user_scoring_accuracy, preferred_sorting: CommentListSortingMethod::Hot };
			users.push(one_user);
		}
		users
	}

	fn calculate_sorting_scores_for_list<'a>(comment_list: &'a mut Vec<Comment<'a>>, scoring_method_to_use: CommentListSortingMethod, use_true_score: bool) -> Vec<(f32, &'a Comment<'a>)> {
		let mut scored_list = vec![];
		for one_comment in comment_list.iter_mut() {
			let score = calculate_sorting_position(&one_comment, scoring_method_to_use, use_true_score);
			if score.is_nan() {
				info!("score is nan");
				info!("{:?}",one_comment.user_scores);
			}
			if !use_true_score { one_comment.perceived_quality = score; }
			scored_list.push((score, &*one_comment))
		};
		scored_list
	}

	fn calculate_sorted_comment_list_score(sorted_comments: &Vec<Comment>, scoring_method: SortScoringMethod) -> f32 {
		let mut score = 0.0;
		for comment_index in 0..sorted_comments.len() {
			info!("index {} quality {}  multiplier {} ", comment_index,sorted_comments.get(comment_index).unwrap().true_quality , (sorted_comments.len() - comment_index) as f32);

			score +=
				match scoring_method {
					SortScoringMethod::Linear => sorted_comments.get(comment_index).unwrap().true_quality * (sorted_comments.len() - comment_index) as f32,
					SortScoringMethod::PlaceSquared => sorted_comments.get(comment_index).unwrap().true_quality * ((sorted_comments.len() - comment_index) as f32).powi(2)
				}
		}
		score
	}

	fn calculate_sorting_position<'a>(comment: &'a Comment<'a>, comment_list_sorting_method: CommentListSortingMethod, use_true_score: bool) -> f32 {
		if use_true_score {
			return comment.true_quality;
		}
		match comment_list_sorting_method {
			CommentListSortingMethod::Top | CommentListSortingMethod::Hot => {
				let (positive_scores, negative_scores) = count_positive_and_negative_user_scores(comment);
				(positive_scores - negative_scores) as f32
			}

			CommentListSortingMethod::New => {
				// new means just give the latest comment the highest score
				(u16::MAX - comment.creation_time) as f32
			}

			CommentListSortingMethod::Best => {
				let returned_count = count_positive_and_negative_user_scores(comment);
				let (positive_scores, negative_scores) = (returned_count.0 as f32, returned_count.1 as f32);
				let zero_count = comment.user_scores.len() as f32 - positive_scores - negative_scores;
				let normal_confidence_interval_95_percent: f32 = 1.95996398454;
				let number_of_scores = comment.user_scores.len() as f32;
				let p_hat = positive_scores / number_of_scores;
				let wilson_score_part1 = p_hat + normal_confidence_interval_95_percent.powi(2) / number_of_scores;
				let wilson_score_under_radical = (p_hat * (1.0 - p_hat) + normal_confidence_interval_95_percent.powi(2) / (4.0 * number_of_scores)) / number_of_scores;
				let wilson_score_part2 = normal_confidence_interval_95_percent * wilson_score_under_radical.sqrt();
				let wilson_score_lower_bound = (wilson_score_part1 - wilson_score_part2) / (1.0 + normal_confidence_interval_95_percent.powi(2) / number_of_scores);
				wilson_score_lower_bound
			}

			CommentListSortingMethod::Controversial => {
				let (positive_scores, negative_scores) = count_positive_and_negative_user_scores(comment);
				// not sure how it's done in reddit, but here we'll use how balanced total votes are
				let fraction_score = ((positive_scores - negative_scores) as f32) / ((positive_scores + negative_scores) as f32);
				let scaled_return = 1.0 - pow(fraction_score, 2) as f32;
				scaled_return
			}
		}
	}

	fn count_positive_and_negative_user_scores(comment: &Comment) -> (u16, u16) {
		let mut positive_scores = 0;
		let mut negative_scores = 0;
		for one_user_score in &comment.user_scores {
			if one_user_score.score > 0.0 {
				positive_scores += 1;
			} else if one_user_score.score < 0.0 {
				negative_scores += 1;
			}
		}
		(positive_scores, negative_scores)
	}

	fn convert_user_comment_score_to_comment_scoring_system(user_comment_score: f32, comment_scoring_method: CommentScoringMethod) -> f32 {
		match comment_scoring_method {
			CommentScoringMethod::ThumbsUpDown => {
				if user_comment_score < -0.5 {
					-1.0
				} else if user_comment_score > 0.5 {
					1.0
				} else {
					0.0
				}
			}
			CommentScoringMethod::ZeroToTen => {
				if user_comment_score < 0.0 {
					5.0 * (1.0 - (user_comment_score / (user_comment_score - 1.0)))
				} else {
					5.0 * (user_comment_score / (user_comment_score + 1.0)) + 5.0
				}
			}
		}
	}

/*	fn generate_optimally_sorted_comments(comments: Vec<Comment>, comment_sort_type: CommentListSortingMethod, use_true_score: bool) -> Vec<Comment> {
		let mut sorted_comments = comments.to_vec();
		sorted_comments.sort_unstable_by(|a, b| {
			let a_score = calculate_sorting_position(a, comment_sort_type, use_true_score);
			let b_score = calculate_sorting_position(b, comment_sort_type, use_true_score);
			b_score.partial_cmp(&a_score).unwrap()
		});
		sorted_comments
	}
*/
	#[cfg(test)]
	mod tests {
		use super::*;

		#[test]
		fn test_calculate_sorting_position_top_true_score() {
			let comment = Comment {
				true_quality: 8.0,
				perceived_quality: 6.5,
				user_scores: vec![],
				creator: &User {
					id: 1,
					reputation: 4.5,
					scoring_accuracy: 0.9,
					preferred_sorting: CommentListSortingMethod::Top,
				},
				creation_time: 100,
			};

			let position = calculate_sorting_position(&comment, CommentListSortingMethod::Top, true);

			// Assuming "Top" sorting favors high true quality
			assert!(position >= 0.0, "Position should be non-negative for Top sorting.");
		}

		#[test]
		fn test_calculate_sorting_position_top_perceived_score() {
			let comment = Comment {
				true_quality: 8.0,
				perceived_quality: 6.5,
				user_scores: vec![],
				creator: &User {
					id: 1,
					reputation: 4.5,
					scoring_accuracy: 0.9,
					preferred_sorting: CommentListSortingMethod::Top,
				},
				creation_time: 100,
			};

			let position = calculate_sorting_position(&comment, CommentListSortingMethod::Top, false);

			// Assuming "Top" sorting favors high true quality
			assert!(position >= 0.0, "Position should be non-negative for Top sorting.");
		}

		#[test]
		fn test_calculate_sorting_position_new_true_score() {
			let comment = Comment {
				true_quality: 5.0,
				perceived_quality: 5.5,
				user_scores: vec![],
				creator: &User {
					id: 2,
					reputation: 3.0,
					scoring_accuracy: 0.7,
					preferred_sorting: CommentListSortingMethod::New,
				},
				creation_time: 200,
			};

			let position = calculate_sorting_position(&comment, CommentListSortingMethod::New, true);

			// Assuming "New" sorting favors recent creation times
			assert!(position >= 0.0, "Position should be non-negative for New sorting.");
		}

		#[test]
		fn test_calculate_sorting_position_new_perceived_score() {
			let comment = Comment {
				true_quality: 5.0,
				perceived_quality: 5.5,
				user_scores: vec![],
				creator: &User {
					id: 2,
					reputation: 3.0,
					scoring_accuracy: 0.7,
					preferred_sorting: CommentListSortingMethod::New,
				},
				creation_time: 200,
			};

			let position = calculate_sorting_position(&comment, CommentListSortingMethod::New, false);

			// Assuming "New" sorting favors recent creation times
			assert!(position >= 0.0, "Position should be non-negative for New sorting.");
		}

		#[test]
		fn test_calculate_sorting_position_best_true_score() {
			let comment = Comment {
				true_quality: 9.0,
				perceived_quality: 9.5,
				user_scores: vec![],
				creator: &User {
					id: 3,
					reputation: 5.0,
					scoring_accuracy: 0.95,
					preferred_sorting: CommentListSortingMethod::Top,
				},
				creation_time: 300,
			};

			let position = calculate_sorting_position(&comment, CommentListSortingMethod::Best, true);

			// Assuming "Best" sorting combines true and perceived quality
			assert!(position >= 0.0, "Position should be non-negative for Best sorting.");
		}

		#[test]
		fn test_calculate_sorting_position_best_perceived_score() {
			let comment = Comment {
				true_quality: 9.0,
				perceived_quality: 9.5,
				user_scores: vec![AssignedScore { score: 0.0, time: 200 }],
				creator: &User {
					id: 3,
					reputation: 5.0,
					scoring_accuracy: 0.95,
					preferred_sorting: CommentListSortingMethod::Top,
				},
				creation_time: 300,
			};

			let position = calculate_sorting_position(&comment, CommentListSortingMethod::Best, false);
			// Assuming "Best" sorting com
			assert!(!position.is_nan(), "Position should be A number for Best sorting.");
		}

		#[test]
		fn test_calculate_sorting_position_controversial_true_score() {
			let comment = Comment {
				true_quality: 7.0,
				perceived_quality: 3.0,  // High variance between true and perceived quality
				user_scores: vec![AssignedScore { score: 1.0, time: 200 }, AssignedScore { score: 0.0, time: 300 }, AssignedScore { score: 1.0, time: 200 }],
				creator: &User {
					id: 4,
					reputation: 2.0,
					scoring_accuracy: 0.6,
					preferred_sorting: CommentListSortingMethod::Hot,
				},
				creation_time: 400,
			};

			let position = calculate_sorting_position(&comment, CommentListSortingMethod::Controversial, true);

			// Assuming "Controversial" sorting favors comments with a high variance between true and perceived quality
			assert!(position == 7.0, "Position should be non-negative for Controversial sorting.");
		}

		#[test]
		fn test_calculate_sorting_position_controversial_perceived_score() {
			let comment = Comment {
				true_quality: 7.0,
				perceived_quality: 3.0,  // High variance between true and perceived quality
				user_scores: vec![AssignedScore { score: 1.0, time: 200 }, AssignedScore { score: -1.0, time: 300 }, AssignedScore { score: 1.0, time: 200 }],
				creator: &User {
					id: 4,
					reputation: 2.0,
					scoring_accuracy: 0.6,
					preferred_sorting: CommentListSortingMethod::Hot,
				},
				creation_time: 400,
			};

			let position = calculate_sorting_position(&comment, CommentListSortingMethod::Controversial, false);
			// Assuming "Controversial" sorting favors comments with a high variance between true and perceived quality
			assert!(position == 0.88888889533, "Position should be .333 squared .");
		}

		#[test]
		fn test_comment_compare_operator() {
			let user1 = User {
				id: 1,
				reputation: 5.0,
				scoring_accuracy: 0.8,
				preferred_sorting: CommentListSortingMethod::Top,
			};

			let user2 = User {
				id: 2,
				reputation: 2.0,
				scoring_accuracy: 0.4,
				preferred_sorting: CommentListSortingMethod::Top,
			};

			let comment1 = Comment {
				true_quality: 5.0,
				perceived_quality: 6.0,
				user_scores: vec![],
				creator: &user1,
				creation_time: 1000,
			};
			let comment2 = Comment { creation_time: 2000, true_quality: 10.0, creator: &user2, ..comment1.clone() };
			let comment3 = Comment { creation_time: 3000, true_quality: 3.0, ..comment1.clone() };
			assert!(comment1 < comment2, "Comment compare operator < 5 less than 10");
			assert!(comment2 > comment3, "Comment compare operator > 10 greater than 3");
			assert!(comment1 >= comment3, "Comment compare operator 1 5 greater than 3");
			assert!(comment1 == comment1.clone(), "Comment equal to it's clone");
			assert!(comment1 != comment3, "Comment not equal operator  5 and 3");
		}
	}

}
