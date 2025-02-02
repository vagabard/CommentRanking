pub mod comment_ranking {
	use std::arch::x86_64::_mm_add_pd;
	use log::LevelFilter;
use rand::seq::SliceRandom;
	use rand::{thread_rng, Rng};
//	use rand_distr::num_traits::{pow, AsPrimitive};
	use rand_distr::Beta;
	use rand_distr::{Distribution, Normal};
	use std::cmp::Ordering;
	use std::fs::File;
	use strum_macros::Display;
	
	use strum_macros::EnumIter;

	use log::info;
	
	const AVERAGE_NUMBER_OF_COMMENTS_VIEWED_BY_USER: f64 = 800.0;
	const TIME_TO_KEEP_NEW_COMMENTS_AT_TOP: u16 = 5;
	const NOISE_LEVEL_FOR_USER_SCORING: f32 = 0.0;
	const SCALE_FOR_USER_ERROR: f32 = 1.0;

	const DO_LOGGING: bool = false ;
	
	static mut LOG_INITIALIZED: bool = false;
	

	#[derive(Clone, Copy, Display)]
	pub enum SortScoringMethod {
		Linear,
		PlaceSquared,
	}
	#[derive(Clone, Copy, Display)]
	pub enum CommentScoringMethod {
		ThumbsUpDown,
		ZeroToTen,
		RawScore,
	}

	#[derive(Clone, Copy, Debug, EnumIter, Display, PartialEq)]
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
		//		reputation: f32,
		scoring_error: f32,
		//		preferred_sorting: CommentListSortingMethod,
		//user_comments: Vec<Comment>
	}

	#[derive(Clone, Copy, Debug)]
	struct AssignedScore {
		score: f32,
//		time: u16,
	}

	#[derive(Clone, Debug)]
	struct Comment {
		true_quality: f32,
		perceived_quality: f32,
		user_scores: Vec<AssignedScore>, // tuple of the score given and when given
//		creator: &'a User,
		creation_time: u16,
	}

	impl<'a> Comment {
		fn add_assigned_score(&mut self, score: AssignedScore) {
			self.user_scores.push(score);
		}
	}

	impl Eq for Comment {}
	impl PartialEq<Self> for Comment {
		fn eq(&self, other: &Self) -> bool {
			self.true_quality.eq(&other.true_quality)
		}
	}
	impl PartialOrd<Self> for Comment {
		fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
			self.true_quality.partial_cmp(&other.true_quality)
		}
	}
	impl Ord for Comment {
		fn cmp(&self, other: &Self) -> Ordering {
			self.true_quality.partial_cmp(&other.true_quality).unwrap()
		}
	}

	pub fn simulate_comments_for_one_topic(number_of_comments: u16, number_of_user_interactions: u16, comment_scoring_method: CommentScoringMethod, comment_list_sorting_type: CommentListSortingMethod, sort_scoring_method: SortScoringMethod,) -> f32 {
		let mut debug_info_to_return = String::new();
		unsafe{
			if DO_LOGGING &&  !LOG_INITIALIZED {
				env_logger::Builder::from_default_env().filter_level(LevelFilter::Info).init();
				LOG_INITIALIZED = true;
			}
		}
		
		let mut rng = thread_rng();
		let users = generate_user_list_for_topic(number_of_user_interactions);
		let make_comment_threshold = number_of_comments as f64 / number_of_user_interactions as f64;

		let mut comment_list_result: Vec<(f32, Comment)> =
			Vec::with_capacity(number_of_comments as usize);
		for current_number_of_views_for_topic in 0..number_of_user_interactions {
			let user_for_this_interaction = users.choose(&mut rng).unwrap();

			let chance_to_make_comment: f64 = rng.gen();
			let random_chance_says_make_comment = make_comment_threshold > chance_to_make_comment;
			let number_of_comments_already_made = comment_list_result.len();
			let space_for_comment_available =	number_of_comments_already_made < number_of_comments as usize;
			let make_a_comment = random_chance_says_make_comment || number_of_comments_already_made < 1;
			if space_for_comment_available && make_a_comment {
				//add comment
				let comment = generate_one_comments(
					&user_for_this_interaction,
					current_number_of_views_for_topic,
				);
				comment_list_result.push((0.5, comment));
			}
			let mut comment_list_copy_to_pass = comment_list_result.to_vec();
			// figure out how the list should be sorted
			let mut position_score_comment_list_perceived: &mut Vec<(f32, Comment)> = calculate_sorting_scores_for_comments_in_list(
				&mut comment_list_copy_to_pass,
				comment_list_sorting_type,
				current_number_of_views_for_topic.into(),
				false,
			);
			// now sort it based on the scores computed
			position_score_comment_list_perceived.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
			//  simulate the user scrolling through the sorted list and voting on comments.
			let user_comments = position_score_comment_list_perceived.iter().map(|(_, comment)| comment.user_scores.clone()).collect::<Vec<Vec<AssignedScore>>>();
			debug_info_to_return.push_str( format!(" assigned comments pre interaction {:?}", user_comments).as_str());
			simulate_one_user_interaction(
				&mut position_score_comment_list_perceived,
				user_for_this_interaction,
				current_number_of_views_for_topic,
				comment_scoring_method,
			);
			let user_comments = position_score_comment_list_perceived.iter().map(|(_, comment)| comment.user_scores.clone()).collect::<Vec<Vec<AssignedScore>>>();
			info!(" assigned comments post interaction {:?}", user_comments);
			comment_list_result = position_score_comment_list_perceived.clone();
		}
		// create a sorted list based on the true quality of the comments
		let mut position_score_comment_list_optimal = comment_list_result.to_vec();
		let position_score_comment_list_optimal =
			calculate_sorting_scores_for_comments_in_list(&mut position_score_comment_list_optimal, comment_list_sorting_type, -1, true);
		position_score_comment_list_optimal
			.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
		info!(
			"ppost scored sorted list {:?}",
			position_score_comment_list_optimal
//			scored_comment_list_to_string(&position_score_comment_list_optimal)
		);

		let maximum_possible_score =
			cumulative_score_for_sorted_list(&position_score_comment_list_optimal, sort_scoring_method);
		debug_info_to_return.push_str( format!(
			"ppost sorted list {:?}",
			position_score_comment_list_optimal
//			comment_list_to_string(&position_score_comment_list_optimal)
		).as_str());
		let mut position_score_comment_list_perceived = comment_list_result.to_vec();
		let position_score_comment_list_perceived =
			calculate_sorting_scores_for_comments_in_list(&mut position_score_comment_list_perceived, comment_list_sorting_type, -1, false);
		position_score_comment_list_perceived
			.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

		let user_score =
			cumulative_score_for_sorted_list(&position_score_comment_list_perceived, sort_scoring_method);
		debug_info_to_return.push_str( format!(
			"max score {} user_score {} ",
			maximum_possible_score, user_score
		).as_str());
		if user_score.is_nan() || maximum_possible_score.is_nan() {
			info!("{}", debug_info_to_return);
			panic!("user score is nan");
		}
		info!("{} {}", user_score, maximum_possible_score);
		user_score / maximum_possible_score
	}

	
	fn comment_list_to_string(comments: &Vec<(f32,Comment)>) -> String {
		let mut comment_list_string = String::new();
		for comment in comments {
			comment_list_string.push_str(&format!("  **COMMENT** {} t {} p {} ctime {}",comment.0, comment.1.true_quality, comment.1.perceived_quality, comment.1.creation_time));
		}
		comment_list_string
	}

	fn simulate_one_user_interaction(
		list_of_comments: &mut Vec<(f32,Comment)>,
		user: &User,
		time_of_viewing: u16,
		comment_scoring_method: CommentScoringMethod,
	) -> () {
		let mut number_of_comments_to_view = thread_rng()
			.sample(Normal::new(AVERAGE_NUMBER_OF_COMMENTS_VIEWED_BY_USER, 1.).unwrap())
			as u16;
		let beta = Beta::new(5.0, 5.0).unwrap();
		number_of_comments_to_view += 1; // make sure at least one comment is viewed, assume every user reads at least the top comment
		if number_of_comments_to_view > list_of_comments.len() as u16 {
			number_of_comments_to_view = list_of_comments.len() as u16;
		}
		info!("number_of_comments_to_view {}", number_of_comments_to_view);
		for comment_index in 0..number_of_comments_to_view {
			//if list_of_comments.len() >= (comment_index + 1) as usize {}
			let one_comment = list_of_comments.get_mut(comment_index as usize).unwrap();
			let scoring_noise = NOISE_LEVEL_FOR_USER_SCORING*(beta.sample(&mut thread_rng()) - 0.5); // beta goes from 0 to 1, we want this centered around 0
			let user_scoring_precision = 1.0; // scoring_noise * user.scoring_accuracy;
			let new_comment_score =
				(1.0+user_scoring_precision) * one_comment.1.true_quality; // multiply how accurate the user is at true quality, and quality.  Then add some noise
			let converted_score = convert_user_comment_score_to_comment_scoring_system(
				new_comment_score,
				comment_scoring_method,
			);
			one_comment.1.add_assigned_score(AssignedScore {
				score: converted_score,
//				time: time_of_viewing,
			})
		}
	}

	fn generate_one_comments(user: &User, time: u16) -> Comment {
		let mut rng = thread_rng();

		// use a beta distribution rather than normal so all scores are between 0 and 1
		let beta = Beta::new(5.0, 5.0).unwrap();
		let comment_score = beta.sample(&mut rng) as f32;
		let one_comment = Comment {
			true_quality: comment_score,
			perceived_quality: 0.0,
			user_scores: Vec::new(),
//			creator: user,
			creation_time: time,
		};
		one_comment
	}

	fn generate_user_list_for_topic(number_of_users_to_create: u16) -> Vec<User> {
		let mut users = Vec::new();
		let mut rng = thread_rng();
		let beta = Beta::new(5.0, 5.0).unwrap();
//		let user_error_file_header = format!("user-errors-{}-", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
//		let filename = format!("{}.csv", user_error_file_header);
//		let file = File::create(&filename).expect("Failed to create file");
//		let mut wtr = csv::Writer::from_writer(file);
		
		for user_index in 0..number_of_users_to_create {
			let user_scoring_error = SCALE_FOR_USER_ERROR* (beta.sample(&mut rng) - 0.5); //center this around 0
//			wtr.write_record(&[user_scoring_error.to_string()]);
//			println!("user_scoring_error {}", user_scoring_error);
			let one_user = User {
				id: user_index,
//				reputation: beta.sample(&mut rng),
				scoring_error: user_scoring_error,
//				preferred_sorting: CommentListSortingMethod::Hot,
			};
			users.push(one_user);
		}
//		wtr.flush().expect("Failed to flush file");
		users
	}

	fn calculate_sorting_scores_for_comments_in_list<'a>(comment_list: &'a mut Vec<(f32, Comment)>, scoring_method_to_use: CommentListSortingMethod, current_time: i32, use_true_score: bool,) -> &'a mut Vec<(f32, Comment)> {
		for one_comment in comment_list.iter_mut() {
			one_comment.0 = calculate_sorting_score_for_one_comment(
				&one_comment.1,
				scoring_method_to_use,
				current_time,
				use_true_score,
			);
			if one_comment.0.is_nan() {
				info!("score is nan");
				info!("{:?}", one_comment.1.user_scores);
			}
			if !use_true_score {
				one_comment.1.perceived_quality = one_comment.0;
			}
		}
		comment_list
	}

	fn cumulative_score_for_sorted_list(
		sorted_comments: &Vec<(f32,Comment)>,
		scoring_method: SortScoringMethod,
	) -> f32 {
		let mut score = 0.0;
		for comment_index in 0..sorted_comments.len() {
			// use the true score for the comment, not the perceived score
			let comment_score_to_use = sorted_comments.get(comment_index).unwrap().1.true_quality;
			let score_increment = match scoring_method {
				SortScoringMethod::Linear => {
					comment_score_to_use* (sorted_comments.len() - comment_index) as f32
				}
				SortScoringMethod::PlaceSquared => {
					comment_score_to_use* ((sorted_comments.len() - comment_index) as f32).powi(2)
				}
			};
			score += score_increment;
			info!(
				"index {} quality {}  multiplier {} score increase {} new score {}",
				comment_index,
				sorted_comments.get(comment_index).unwrap().0,
				(sorted_comments.len() - comment_index) as f32,
				score_increment,
				score
			);
		}
		score
	}

	fn check_for_external_reason_to_boost_comment(comment: &Comment, current_time: i32) -> f32 {
		if current_time > 0
			&& (comment.creation_time as i32 - current_time) < TIME_TO_KEEP_NEW_COMMENTS_AT_TOP as i32
		{
			// if current time is less than 0, then ignore whether the comment is new.
			return f32::MAX; // make sure new comments are at the top
		}
		f32::MIN
	}

	fn calculate_sorting_score_for_one_comment(
		comment: &Comment,
		comment_list_sorting_method: CommentListSortingMethod,
		current_time: i32,
		use_true_score: bool,
	) -> f32 {
		//for the true score, return the real quality.  Unless it's new, then we'll return creation time whether or not it's true score
		if use_true_score && comment_list_sorting_method != CommentListSortingMethod::New {
			return comment.true_quality;
		}
		if comment_list_sorting_method != CommentListSortingMethod::New {
			let external_reason_to_boost_comment =
				check_for_external_reason_to_boost_comment(comment, current_time);
			if external_reason_to_boost_comment > f32::MIN {
				return external_reason_to_boost_comment;
			}
		}
		return match comment_list_sorting_method {
				CommentListSortingMethod::Top | CommentListSortingMethod::Hot => {
					let (positive_scores, negative_scores) =
						count_positive_and_negative_user_scores(&comment);
					(positive_scores - negative_scores) as f32
				}

				CommentListSortingMethod::New => {
					// new means just give the latest comment the highest score.
					comment.creation_time as f32
				}

				CommentListSortingMethod::Best => {
					let returned_count = count_positive_and_negative_user_scores(&comment);
					let (positive_scores, _negative_scores) =
						(returned_count.0 as f32, returned_count.1 as f32);
					//let zero_count = comment.1.user_scores.len() as f32 - positive_scores - negative_scores;
					let normal_confidence_interval_95_percent: f32 = 1.95996398454;
					let mut number_of_scores = comment.user_scores.len() as f32;
					let mut p_hat = positive_scores / number_of_scores;
					if p_hat.is_nan(){
						p_hat = 1.0; // if there are no scores, then just return 1.0
						number_of_scores = 1.0; // pretend there's a score to avoid divide by zero
					}
					let wilson_score_part1 =
						p_hat + normal_confidence_interval_95_percent.powi(2) / number_of_scores;
					let wilson_score_under_radical = (p_hat * (1.0 - p_hat)
						+ normal_confidence_interval_95_percent.powi(2) / (4.0 * number_of_scores))
						/ number_of_scores;
					let wilson_score_part2 =
						normal_confidence_interval_95_percent * wilson_score_under_radical.sqrt();
					let wilson_score_lower_bound = (wilson_score_part1 - wilson_score_part2)
						/ (1.0 + normal_confidence_interval_95_percent.powi(2) / number_of_scores);
					info!(
					"SortBest pos scores {} negative scores {} wilson_part_1 {}  wilson_radical {} Wilson_part_2 {} wilson_score_lower_bound {}",
						returned_count.0, returned_count.1, wilson_score_part1, wilson_score_under_radical, wilson_score_part2, wilson_score_lower_bound
					);
					
					wilson_score_lower_bound
				}
				// this is crap, don't use it.
				CommentListSortingMethod::Controversial => {
					let (positive_scores, negative_scores) =
						count_positive_and_negative_user_scores(&comment);
					let fraction_score = ((positive_scores as i32 - negative_scores as i32) as f32)
						/ ((positive_scores + negative_scores) as f32);
					let scaled_return = 1.0 - (fraction_score * fraction_score);
					scaled_return
				}
		}
	}

	fn count_positive_and_negative_user_scores(comment: &Comment) -> (u16, u16) {
		let mut positive_scores = 0;
		let mut negative_scores = 0;
		for one_user_score in &comment.user_scores {
			info!("user score {:?}", one_user_score);
			if one_user_score.score > 0.0 {
				positive_scores += 1;
			} else if one_user_score.score < 0.0 {
				negative_scores += 1;
			}
		}
		(positive_scores, negative_scores)
	}

	fn convert_user_comment_score_to_comment_scoring_system(
		user_comment_score: f32,
		comment_scoring_method: CommentScoringMethod,
	) -> f32 {
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
			CommentScoringMethod::RawScore => {
				user_comment_score
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
				/*
				creator: &User {
					id: 1,
//					reputation: 4.5,
					scoring_accuracy: 0.9,
//					preferred_sorting: CommentListSortingMethod::Top,
				},
				*/
				
				creation_time: 100,
			};

			let position = calculate_sorting_score_for_one_comment(
				&comment,
				CommentListSortingMethod::Top,
				100,
				true,
			);

			// Assuming "Top" sorting favors high true quality
			assert!(
				position >= 0.0,
				"Position should be non-negative for Top sorting."
			);
		}

		#[test]
		fn test_calculate_sorting_position_top_perceived_score() {
			let comment = Comment {
				true_quality: 8.0,
				perceived_quality: 6.5,
				user_scores: vec![],
				/*
				creator: &User {
					id: 1,
//					reputation: 4.5,
					scoring_accuracy: 0.9,
//					preferred_sorting: CommentListSortingMethod::Top,
				},
				*/
				
				creation_time: 100,
			};

			let position = calculate_sorting_score_for_one_comment(
				&comment,
				CommentListSortingMethod::Top,
				100,
				false,
			);

			// Assuming "Top" sorting favors high true quality
			assert!(
				position >= 0.0,
				"Position should be non-negative for Top sorting."
			);
		}

		#[test]
		fn test_calculate_sorting_position_new_true_score() {
			let comment = Comment {
				true_quality: 5.0,
				perceived_quality: 5.5,
				user_scores: vec![],
				/*
				creator: &User {
					id: 2,
//					reputation: 3.0,
					scoring_accuracy: 0.7,
//					preferred_sorting: CommentListSortingMethod::New,
				},
				
				 */
				creation_time: 200,
			};

			let position = calculate_sorting_score_for_one_comment(
				&comment,
				CommentListSortingMethod::New,
				300,
				true,
			);

			// Assuming "New" sorting favors recent creation times
			assert!(
				position >= 0.0,
				"Position should be non-negative for New sorting."
			);
		}

		#[test]
		fn test_calculate_sorting_position_new_perceived_score() {
			let comment = Comment {
				true_quality: 5.0,
				perceived_quality: 5.5,
				user_scores: vec![],
				/*
				creator: &User {
					id: 2,
//					reputation: 3.0,
					scoring_accuracy: 0.7,
//					preferred_sorting: CommentListSortingMethod::New,
				},
				
				 */
				creation_time: 200,
			};

			let position = calculate_sorting_score_for_one_comment(
				&comment,
				CommentListSortingMethod::New,
				300,
				false,
			);

			// Assuming "New" sorting favors recent creation times
			assert!(
				position >= 0.0,
				"Position should be non-negative for New sorting."
			);
		}

		#[test]
		fn test_calculate_sorting_position_best_true_score() {
			let comment = Comment {
				true_quality: 9.0,
				perceived_quality: 9.5,
				user_scores: vec![],
				/*
				creator: &User {
					id: 3,
//					reputation: 5.0,
					scoring_accuracy: 0.95,
//					preferred_sorting: CommentListSortingMethod::Top,
				},
				
				 */
				creation_time: 300,
			};

			let position = calculate_sorting_score_for_one_comment(
				&comment,
				CommentListSortingMethod::Best,
				100,
				true,
			);

			// Assuming "Best" sorting combines true and perceived quality
			assert!(
				position >= 0.0,
				"Position should be non-negative for Best sorting."
			);
		}

		#[test]
		fn test_calculate_sorting_position_best_perceived_score() {
			let comment = Comment {
				true_quality: 9.0,
				perceived_quality: 9.5,
				user_scores: vec![AssignedScore {
					score: 0.0,
//					time: 200,
				}],
				/*
				creator: &User {
					id: 3,
//					reputation: 5.0,
					scoring_accuracy: 0.95,
//					preferred_sorting: CommentListSortingMethod::Top,
				},
				
				 */
				creation_time: 300,
			};

			let position = calculate_sorting_score_for_one_comment(
				&comment,
				CommentListSortingMethod::Best,
				100,
				false,
			);
			// Assuming "Best" sorting com
			assert!(
				!position.is_nan(),
				"Position should be A number for Best sorting."
			);
		}

		#[test]
		fn test_calculate_sorting_position_controversial_true_score() {
			let comment = Comment {
				true_quality: 7.0,
				perceived_quality: 3.0, // High variance between true and perceived quality
				user_scores: vec![
					AssignedScore {
						score: 1.0,
//						time: 200,
					},
					AssignedScore {
						score: 0.0,
//						time: 300,
					},
					AssignedScore {
						score: 1.0,
//						time: 200,
					},
				],
				/*creator: &User {
					id: 4,
//					reputation: 2.0,
					scoring_accuracy: 0.6,
//					preferred_sorting: CommentListSortingMethod::Hot,
				},
				
				 */
				creation_time: 400,
			};

			let position = calculate_sorting_score_for_one_comment(
				&comment,
				CommentListSortingMethod::Controversial,
				100,
				true,
			);

			// Assuming "Controversial" sorting favors comments with a high variance between true and perceived quality
			assert!(
				position == 7.0,
				"Position should be non-negative for Controversial sorting."
			);
		}

		#[test]
		fn test_calculate_sorting_position_controversial_perceived_score() {
			let comment = Comment {
				true_quality: 7.0,
				perceived_quality: 3.0, // High variance between true and perceived quality
				user_scores: vec![
					AssignedScore {
						score: 1.0,
//						time: 200,
					},
					AssignedScore {
						score: -1.0,
//						time: 300,
					},
					AssignedScore {
						score: 1.0,
//						time: 200,
					},
				],
				/*
				creator: &User {
					id: 4,
//					reputation: 2.0,
					scoring_accuracy: 0.6,
//					preferred_sorting: CommentListSortingMethod::Hot,
				},
				
				 */
				creation_time: 400,
			};
			let mut comment_tuple = (0.0, comment);
			let position = calculate_sorting_score_for_one_comment(
				&comment_tuple.1,
				CommentListSortingMethod::Controversial,
				100,
				false,
			);
			// Assuming "Controversial" sorting favors comments with a high variance between true and perceived quality
			assert!(
				position == 0.88888889533,
				"Position should be .333 squared ."
			);
		}

		#[test]
		fn test_comment_compare_operator() {
			let user1 = User {
				id: 1,
//				reputation: 5.0,
				scoring_error: 0.8,
//				preferred_sorting: CommentListSortingMethod::Top,
			};

			let user2 = User {
				id: 2,
//				reputation: 2.0,
				scoring_error: 0.4,
//				preferred_sorting: CommentListSortingMethod::Top,
			};

			let comment1 = Comment {
				true_quality: 5.0,
				perceived_quality: 6.0,
				user_scores: vec![],
//				creator: &user1,
				creation_time: 1000,
			};
			let comment2 = Comment {
				creation_time: 2000,
				true_quality: 10.0,
//				creator: &user2,
				..comment1.clone()
			};
			let comment3 = Comment {
				creation_time: 3000,
				true_quality: 3.0,
				..comment1.clone()
			};
			assert!(
				comment1 < comment2,
				"Comment compare operator < 5 less than 10"
			);
			assert!(
				comment2 > comment3,
				"Comment compare operator > 10 greater than 3"
			);
			assert!(
				comment1 >= comment3,
				"Comment compare operator 1 5 greater than 3"
			);
			assert!(comment1 == comment1.clone(), "Comment equal to it's clone");
			assert!(comment1 != comment3, "Comment not equal operator  5 and 3");
		}
	}

}
