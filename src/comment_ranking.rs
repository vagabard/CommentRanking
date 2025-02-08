pub mod comment_ranking {
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

	const AVERAGE_NUMBER_OF_COMMENTS_VIEWED_BY_USER: f64 = 0.0;
	const TIME_TO_KEEP_NEW_COMMENTS_AT_TOP: u16 =5;
	const NOISE_LEVEL_FOR_USER_SCORING: f32 =1.0;  // 0.0 is no noise, 1.0 is lots of noise
	const SCALE_FOR_USER_ERROR: f32 = 1.0; // 0.0 is no error, 1.0 is lots of error

	const THRESHOLD_FOR_VOTING_ON_COMMENT: f32 = 0.0;// if the score is within this of the middle value, assume it's not voted on.

//	const NORMALIZE_SCORE_FROM_ZERO_TO_ONE: bool = true;

	const DO_LOGGING: bool = false ;
	
	static mut LOG_INITIALIZED: bool = false;
	

	#[derive(Clone, Copy, Display)]
	pub enum SortScoringMethod {
		Linear,
		PlaceSquared,
		NormalizedLinear,
		NormalizedPlaceSquared,
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

	pub enum _CommentAppearanceDistribution{
		Uniform,
		Normal,
		Beta,
		Custom,
	}

	#[derive(Clone, Copy, Display)]
	pub enum LowScoreMemberHandling {
		Ignore,
		Flat_Percent_Chance,
		Proportional_To_Score_Chance,

	}


	#[derive(Copy, Clone, Debug)]
	struct User {
		id: u32,
		//		reputation: f32,
		scoring_accuracy: f32,
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
		creation_time: u32,
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

	pub fn simulate_comments_for_one_topic(number_of_comments: u32, number_of_user_interactions: u32, comment_scoring_method: CommentScoringMethod, comment_list_sorting_type: CommentListSortingMethod, sort_scoring_method: SortScoringMethod, checkpoint_progress_interaction_count:u32, method_to_handle_low_scoring_members: LowScoreMemberHandling) -> Vec<(u32, f32)> {
		//let mut debug_info_to_return = String::new();
		unsafe{
			if DO_LOGGING &&  !LOG_INITIALIZED {
				env_logger::Builder::from_default_env().filter_level(LevelFilter::Info).init();
				LOG_INITIALIZED = true;
			}
		}
		
		let mut rng = thread_rng();
		let users = generate_user_list_for_topic(number_of_user_interactions);
		let mut score_to_interaction_number_map: Vec<(u32, f32)> = Vec::with_capacity((number_of_user_interactions / checkpoint_progress_interaction_count) as usize);
		let mut comment_list_result: Vec<(f32, Comment)> =
			Vec::with_capacity(number_of_comments as usize);
		/*
		let mut comments_per_user_interaction_wtr = create_cvs_writer_for_debugging("number_of_comments_per_user_interaction");
		let mut noise_per_user_score_wtr = create_cvs_writer_for_debugging("noise_distortion");
		let mut user_scores_wtr = create_cvs_writer_for_debugging("user_scores");
		let mut noisy_per_user_score_wtr = create_cvs_writer_for_debugging("noisy_scores");
		let mut comment_making_time_wtr = create_cvs_writer_for_debugging("comment_making_time");
		let mut user_score_normalized_wtr = _create_cvs_writer_for_debugging("score_user_normalized");
		let mut optimum_score_normalized_wtr = _create_cvs_writer_for_debugging("score_optimum_normalized");
		*/
		for current_number_of_views_for_topic in 0..number_of_user_interactions {
			let user_for_this_interaction = users.choose(&mut rng).unwrap();

			let number_of_comments_already_made = comment_list_result.len();
			// this is a basic threshold, for making a comment, based off of uniform distribution of remaining comments in the remaining interactions.
			let make_comment_threshold_uniform = (number_of_comments - number_of_comments_already_made as u32) as f64 / (number_of_user_interactions-current_number_of_views_for_topic) as f64;
			//this should make it a normalized power distribution.
			let beta = Beta::new(1.0, 5.0).unwrap();
			let chance_to_make_comment: f64 = beta.sample(&mut thread_rng());
			let random_chance_says_make_comment = make_comment_threshold_uniform > chance_to_make_comment;

			let space_for_comment_available =	number_of_comments_already_made < number_of_comments as usize;
			let make_a_comment = random_chance_says_make_comment || number_of_comments_already_made < 1;
			if space_for_comment_available && make_a_comment {
				// we're making a comment, record the time
				//comment_making_time_wtr.write_record(&[&current_number_of_views_for_topic.to_string()]).unwrap();
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
				comment_scoring_method,
				comment_list_sorting_type,
				current_number_of_views_for_topic as i32,
				method_to_handle_low_scoring_members,
				false,
			);
			// now sort it based on the scores computed
			position_score_comment_list_perceived.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
			//  simulate the user scrolling through the sorted list and voting on comments.
			//let user_comments = position_score_comment_list_perceived.iter().map(|(_, comment)| comment.user_scores.clone()).collect::<Vec<Vec<AssignedScore>>>();
			//info!(" assigned comments pre interaction {:?}", user_comments);
			let (_number_of_comments_viewed, _noise_for_scores, _user_scores_of_comments, _noisy_scores) = simulate_one_user_interaction(
				&mut position_score_comment_list_perceived,
				user_for_this_interaction,
				current_number_of_views_for_topic,
				comment_scoring_method,
			);
			/*
			comments_per_user_interaction_wtr.write_record(&[current_number_of_views_for_topic.to_string(), number_of_comments_viewed.to_string()]).unwrap();
			for one_distortion in noise_for_scores {
				noise_per_user_score_wtr.write_record(&[one_distortion.to_string()]).unwrap();
			}
			for one_user_score in user_scores_of_comments {
				user_scores_wtr.write_record(&[one_user_score.to_string()]).unwrap();
			}
			for one_noise in noisy_scores {
				noisy_per_user_score_wtr.write_record(&[one_noise.to_string()]).unwrap();
			}

			let user_comments = position_score_comment_list_perceived.iter().map(|(_, comment)| comment.user_scores.clone()).collect::<Vec<Vec<AssignedScore>>>();
			info!(" assigned comments post interaction {:?}", user_comments);
			 */
			comment_list_result = position_score_comment_list_perceived.clone();
			if (current_number_of_views_for_topic % checkpoint_progress_interaction_count) == 0  && current_number_of_views_for_topic > 0 {
//				println!("len before add {}  view index {}",score_to_interaction_number_map.len(),  current_number_of_views_for_topic );
				let (user_to_optimal_sort_ratio, max_scores, user_scores) = calculate_ratio_of_computed_sort_to_optimal_sort(
					comment_list_sorting_type,
					comment_scoring_method,
					sort_scoring_method,
					&comment_list_result,
					method_to_handle_low_scoring_members,
				);
				score_to_interaction_number_map.push(( current_number_of_views_for_topic, user_to_optimal_sort_ratio));
			}
//			print_score_and_truescore_pair_for_comment(position_score_comment_list_perceived.clone());
			/*
				for one_user_vector in user_scores {
					user_score_normalized_wtr.write_record(&[one_user_vector.to_string()]).unwrap();
				}
				for one_optimum_vector in max_scores {
					optimum_score_normalized_wtr.write_record(&[one_optimum_vector.to_string()]).unwrap();
				}
			comments_per_user_interaction_wtr.flush().unwrap();
			noise_per_user_score_wtr.flush().unwrap();
			user_scores_wtr.flush().unwrap();
			noisy_per_user_score_wtr.flush().unwrap();
			comment_making_time_wtr.flush().unwrap();
			user_score_normalized_wtr.flush().unwrap();
			optimum_score_normalized_wtr.flush().unwrap();
			 */
		}
		score_to_interaction_number_map
	}

	fn _print_score_and_truescore_pair_for_comment(stuff: Vec<(f32, Comment)>){
		for one_pair in stuff {
			println!("score: {} pc: {} true: {} \n", one_pair.0, one_pair.1.perceived_quality, one_pair.1.true_quality);
		}

	}

	fn calculate_ratio_of_computed_sort_to_optimal_sort(
		comment_list_sorting_type: CommentListSortingMethod,
		comment_scoring_method: CommentScoringMethod,
		sort_scoring_method: SortScoringMethod,
		comment_list_result: &Vec<(f32, Comment)>,
		method_to_handle_low_score_members:LowScoreMemberHandling,
	) -> (f32, Vec<f32>, Vec<f32>) {
		// create a sorted list based on the true quality of the comments
		let mut position_score_comment_list_optimal = comment_list_result.to_vec();
		let position_score_comment_list_optimal =
			calculate_sorting_scores_for_comments_in_list(&mut position_score_comment_list_optimal, comment_scoring_method, comment_list_sorting_type,-1,method_to_handle_low_score_members, true);
		position_score_comment_list_optimal
			.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
		info!(
			"ppost scored sorted list {:?}",
			position_score_comment_list_optimal
		);

		let maximum_possible_score =
			cumulative_score_for_sorted_list(&position_score_comment_list_optimal, sort_scoring_method);
//		println!("optimal {}",maximum_possible_score.0);
//		print_score_and_truescore_pair_for_comment(position_score_comment_list_optimal.clone());

		let mut position_score_comment_list_minimal = comment_list_result.to_vec();
		let position_score_comment_list_minimal =
			calculate_sorting_scores_for_comments_in_list(&mut position_score_comment_list_minimal, comment_scoring_method, comment_list_sorting_type,-1,method_to_handle_low_score_members, true);
		position_score_comment_list_minimal
			.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
		info!(
			"ppost scored sorted list {:?}",
			position_score_comment_list_minimal
		);

		let minimal_possible_score =
			cumulative_score_for_sorted_list(&position_score_comment_list_minimal, sort_scoring_method);
//		println!("minimal {}",minimal_possible_score.0);
//		print_score_and_truescore_pair_for_comment(position_score_comment_list_minimal.clone());


		let mut debug_info_to_return = String::new();
		debug_info_to_return.push_str(format!(
			"ppost sorted list {:?}",
			position_score_comment_list_optimal
		).as_str());
		let mut position_score_comment_list_perceived = comment_list_result.to_vec();
		let position_score_comment_list_perceived =
			calculate_sorting_scores_for_comments_in_list(&mut position_score_comment_list_perceived, comment_scoring_method, comment_list_sorting_type, -1,method_to_handle_low_score_members, false);
		position_score_comment_list_perceived
			.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
		let user_score =
			cumulative_score_for_sorted_list(&position_score_comment_list_perceived, sort_scoring_method);
//		println!("estimated {}", user_score.0);
//		print_score_and_truescore_pair_for_comment(position_score_comment_list_perceived.clone());
		debug_info_to_return.push_str(format!(
			"max score {}  user_score {} ",
			maximum_possible_score.0, user_score.0
		).as_str());

		if user_score.0.is_nan() || maximum_possible_score.0.is_nan() {
//			print!("{}", debug_info_to_return);
			panic!("user score is nan");
		}
		let mut ratio_to_return= match(sort_scoring_method){
			SortScoringMethod::NormalizedPlaceSquared|SortScoringMethod::NormalizedLinear=>{
				let score_range = maximum_possible_score.0 - minimal_possible_score.0;
				(user_score.0 - minimal_possible_score.0)/score_range
			}
			_=> {
				user_score.0 / maximum_possible_score.0
			}

		} ;
//		println!("user ratio {} \n\n\n", ratio_to_return);
		info!("{} {}", user_score.0, maximum_possible_score.0);
		(ratio_to_return, maximum_possible_score.1.clone(),user_score.1.clone())
	}
	
	fn simulate_one_user_interaction(
		list_of_comments: &mut Vec<(f32,Comment)>,
		user: &User,
		_time_of_viewing: u32,
		comment_scoring_method: CommentScoringMethod,
	) -> (u16,Vec<f32>,Vec<f32>,Vec<f32>) {
		let mut number_of_comments_to_view = thread_rng()
			.sample(Normal::new(AVERAGE_NUMBER_OF_COMMENTS_VIEWED_BY_USER, 1.).unwrap())
			as u16;
		let beta = Beta::new(2.0, 2.0).unwrap();
		number_of_comments_to_view += 1; // make sure at least one comment is viewed, assume every user reads at least the top comment
		if number_of_comments_to_view > list_of_comments.len() as u16 {
			number_of_comments_to_view = list_of_comments.len() as u16;
		}
		info!("number_of_comments_to_view {}", number_of_comments_to_view);
		let mut noise_distortion: Vec<f32> = Vec::new();
		let mut user_scores: Vec<f32> = Vec::new();
		let mut user_scores_with_noise: Vec<f32> = Vec::new();
		for comment_index in 0..number_of_comments_to_view {
			//if list_of_comments.len() >= (comment_index + 1) as usize {}
			let one_comment = list_of_comments.get_mut(comment_index as usize).unwrap();
			let scoring_distortion = 1.0 - NOISE_LEVEL_FOR_USER_SCORING*(beta.sample(&mut thread_rng()) - 0.5); // beta goes from 0 to 1, we want this centered around 0
			noise_distortion.push(scoring_distortion);
			let user_score_for_comment = one_comment.1.true_quality*user.scoring_accuracy; // add a per user scoring error to the true quality
			user_scores.push(user_score_for_comment);
			let user_score_with_noise = user_score_for_comment * scoring_distortion;
			user_scores_with_noise.push(user_score_with_noise);
			let converted_score = convert_user_comment_score_to_comment_scoring_system(
				user_score_with_noise,
				comment_scoring_method,
			);
			one_comment.1.add_assigned_score(AssignedScore {
				score: converted_score,
//				time: time_of_viewing,
			})
		}
		(number_of_comments_to_view, noise_distortion, user_scores, user_scores_with_noise)
	}

	fn generate_one_comments(_user: &User, time: u32) -> Comment {
		let mut rng = thread_rng();

		// use a beta distribution rather than normal so all scores are between 0 and 1
		let beta = Beta::new(2.0, 2.0).unwrap();
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

	fn generate_user_list_for_topic(number_of_users_to_create: u32) -> Vec<User> {
		let mut users = Vec::new();
		let mut rng = thread_rng();
		let beta = Beta::new(5.0, 5.0).unwrap();
//		create_cvs_writer_for_debugging("user_creation");
		for user_index in 0..number_of_users_to_create {
			let user_scoring_correctness = 1.0 + SCALE_FOR_USER_ERROR* (beta.sample(&mut rng) - 0.5); //center this around 1
//			wtr.write_record(&[user_scoring_error.to_string()]);
//			println!("user_scoring_error {}", user_scoring_error);
			let one_user = User {
				id: user_index,
//				reputation: beta.sample(&mut rng),
				scoring_accuracy: user_scoring_correctness,
//				preferred_sorting: CommentListSortingMethod::Hot,
			};
			users.push(one_user);
		}
//		wtr.flush().expect("Failed to flush file");
		users
	}

	fn _create_cvs_writer_for_debugging(filename_prefix: &str) -> csv::Writer<File> {
		let filename = format!("{}-{}.csv", filename_prefix, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
		let file = File::create(&filename).expect("Failed to create file");
		let mut wtr = csv::Writer::from_writer(file);
		wtr
	}

	fn calculate_sorting_scores_for_comments_in_list<'a>(comment_list: &'a mut Vec<(f32, Comment)>, comment_scoring_method: CommentScoringMethod,  scoring_method_to_use: CommentListSortingMethod, current_time: i32, method_to_handle_low_score_members: LowScoreMemberHandling, use_true_score: bool,) -> &'a mut Vec<(f32, Comment)> {
		for one_comment in comment_list.iter_mut() {
			let debug_string:String;
			(one_comment.0, debug_string) = calculate_sorting_score_for_one_comment(
				&one_comment.1,
				scoring_method_to_use,
				comment_scoring_method,
				current_time,
				method_to_handle_low_score_members,
				use_true_score,
			);
			if one_comment.0.is_nan() {
				println!("something broke, score is nan debug {} ", debug_string);
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
	) -> (f32, Vec<f32>) {
		let mut score = 0.0;
		let mut minimum_score_for_normalization = 1.0;
		let mut maximum_score_for_normalization = 0.0;
		let mut score_range_for_normalization = 0.0;
		match scoring_method {
			SortScoringMethod::NormalizedLinear | SortScoringMethod::NormalizedPlaceSquared => {
				for comment_index in 0..sorted_comments.len() {
					// use the true score for the comment, not the perceived score
					let comment_score_to_use = sorted_comments.get(comment_index).unwrap().1.true_quality;
					if comment_score_to_use < minimum_score_for_normalization {
						minimum_score_for_normalization = comment_score_to_use;
					}
					if comment_score_to_use > maximum_score_for_normalization {
						maximum_score_for_normalization = comment_score_to_use;
					}
//					println!("comment score {}", comment_score_to_use);
				}
				score_range_for_normalization = maximum_score_for_normalization - minimum_score_for_normalization;
			}
			_ => {}
		}
		let mut normalized_scores:Vec<f32> =  Vec::with_capacity(sorted_comments.len());
//		println!("score range: {} max score {} min score {}", score_range_for_normalization, maximum_score_for_normalization, minimum_score_for_normalization);
		for comment_index in 0..sorted_comments.len() {
			// use the true score for the comment, not the perceived score
			let comment_score_to_use = sorted_comments.get(comment_index).unwrap().1.true_quality;
			let score_to_use = match scoring_method {
				SortScoringMethod::NormalizedLinear| SortScoringMethod::NormalizedPlaceSquared => {
					if sorted_comments.len() > 1 {
						(comment_score_to_use-minimum_score_for_normalization)/score_range_for_normalization
					}else {
						// if there's only one entry , just use the score
						comment_score_to_use
					}
				}
				_ => comment_score_to_use,
			};
			let score_increment = match scoring_method {
				SortScoringMethod::Linear| SortScoringMethod::NormalizedLinear => {
					score_to_use* (sorted_comments.len() - comment_index) as f32
				}
				SortScoringMethod::PlaceSquared| SortScoringMethod::NormalizedPlaceSquared => {
					score_to_use* ((sorted_comments.len() - comment_index) as f32).powi(2)
				}
			};
//			println!("score increement {}", score_increment);
			normalized_scores.push(score_increment);
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
		(score,  normalized_scores)
	}

	fn check_for_external_reason_to_boost_comment(comment: &Comment, current_time: i32, how_to_handle_low_score_elements:LowScoreMemberHandling) -> f32 {
		let value_to_return_as_max = 1.0; // was using f32::MAX but had some overrun problems.
		let comment_age = current_time - comment.creation_time as i32;
		if current_time > 0
			&& comment_age < TIME_TO_KEEP_NEW_COMMENTS_AT_TOP as i32
		{
			// if current time is less than 0, then ignore whether the comment is new.
			return comment_age as f32 / current_time as f32; // make sure new comments are at the top
		}
		let random_chance = thread_rng().gen_range(0.0..1.0);
		match how_to_handle_low_score_elements {
			LowScoreMemberHandling::Ignore => return 0.0,
			LowScoreMemberHandling::Flat_Percent_Chance => {
				//quick and dirty, give a chance an old comment will be raised.
				if random_chance< 0.02{
					// we want this close to 1 and unique.  Rust gets upset sorting if 2 values are the same
					return value_to_return_as_max - comment.true_quality.powi(5) as f32;
				}else {
					return 0.0;
				}
			}
			LowScoreMemberHandling::Proportional_To_Score_Chance => {
				if random_chance < comment.perceived_quality / 100.0 {
					return value_to_return_as_max - comment.true_quality.powi(5) as f32;
				} else {
					return 0.0;
				}
			}
		}


		0.0
	}

	fn calculate_sorting_score_for_one_comment(
		comment: &Comment,
		comment_list_sorting_method: CommentListSortingMethod,
		comment_scoring_method: CommentScoringMethod,
		current_time: i32,
		method_to_handle_low_score_members: LowScoreMemberHandling,
		use_true_score: bool,
	) -> (f32,String) {
		//for the true score, return the real quality.  Unless it's new, then we'll return creation time whether or not it's true score
		if use_true_score && comment_list_sorting_method != CommentListSortingMethod::New {
			return (comment.true_quality, String::new());
		}
		if comment_list_sorting_method != CommentListSortingMethod::New  && current_time > 0{// current time is greater than zero when we are sorting for viewing.   current time is -1 when sorting to evaluate how well it worked.  We don't want random in that case.
			let external_reason_to_boost_comment =
				check_for_external_reason_to_boost_comment(comment, current_time,method_to_handle_low_score_members);
			if external_reason_to_boost_comment > 0.0 {
				return (external_reason_to_boost_comment, String::new());
			}
		}
		return match comment_list_sorting_method {
				CommentListSortingMethod::Top | CommentListSortingMethod::Hot => {
					let (positive_scores, negative_scores, _positive_total, _negative_total) =
						count_and_sum_positive_and_negative_user_scores(&comment);
					((positive_scores - negative_scores) as f32, String::new())
				}

				CommentListSortingMethod::New => {
					// new means just give the latest comment the highest score.
					(comment.creation_time as f32, String::new())
				}

				CommentListSortingMethod::Best => {
					let returned_count = count_and_sum_positive_and_negative_user_scores(&comment);
					let (positive_scores, _negative_scores) = match comment_scoring_method {
						// if using a raw score, return the precise total.
						CommentScoringMethod::RawScore => {
							(returned_count.2, returned_count.3)
						}
						// if using a thumbs up/down score, return the count of positive scores.
						_ => {
							(returned_count.0 as f32, returned_count.1 as f32)
						}
					};
					//	(returned_count.0 as f32, returned_count.1 as f32);
					//let zero_count = comment.1.user_scores.len() as f32 - positive_scores - negative_scores;
					let normal_confidence_interval_95_percent: f32 = 1.95996398454;
					let mut number_of_scores = match comment_scoring_method {
						// if using a raw score, return the precise total.
						CommentScoringMethod::RawScore => {
							returned_count.2+ returned_count.3
						}
						// if using a thumbs up/down score, return the count of positive scores.
						_ => {comment.user_scores.len() as f32
						}
					};
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
					let wilson_score_under_radical_root = wilson_score_under_radical.sqrt();
					let wilson_score_part2 = normal_confidence_interval_95_percent * wilson_score_under_radical_root;
					let wilson_score_lower_bound = (wilson_score_part1 - wilson_score_part2)
						/ (1.0 + normal_confidence_interval_95_percent.powi(2) / number_of_scores);
					info!(
					"SortBest pos scores {} negative scores {} wilson_part_1 {}  wilson_radical {} Wilson_part_2 {} wilson_score_lower_bound {}",
						returned_count.0, returned_count.1, wilson_score_part1, wilson_score_under_radical, wilson_score_part2, wilson_score_lower_bound
					);
					let debug_string = format!("positive scores {} numscore {} phat {} wilson 1 {} wilson under rad {} wilson under rad root {} wilson 2 {}",positive_scores, number_of_scores,p_hat,wilson_score_part1,wilson_score_under_radical,wilson_score_under_radical_root, wilson_score_part2);
					(wilson_score_lower_bound, debug_string)
				}
				// this is crap, don't use it.
				CommentListSortingMethod::Controversial => {
					let (positive_scores, negative_scores, _positive_total, _negative_total) =
						count_and_sum_positive_and_negative_user_scores(&comment);
					let fraction_score = ((positive_scores as i32 - negative_scores as i32) as f32)
						/ ((positive_scores + negative_scores) as f32);
					let scaled_return = 1.0 - (fraction_score * fraction_score);
					(scaled_return, String::new())
				}
		}
	}

	fn count_and_sum_positive_and_negative_user_scores(comment: &Comment) -> (u32, u32, f32,f32) {
		let mut positive_score_count :u32 = 0;
		let mut negative_score_count:u32 = 0;
		let mut positive_score_total = 0.0;
		let mut negative_score_total = 0.0;
		for one_user_score in &comment.user_scores {
			//scores go from 0 to 1, so
			info!("user score {:?}", one_user_score);
			if one_user_score.score > 0.50 {
				positive_score_count += 1;
				positive_score_total += one_user_score.score;
			} else if one_user_score.score < 0.50 {
				negative_score_count += 1;
				negative_score_total += one_user_score.score;
			}
		}
		(positive_score_count, negative_score_count, positive_score_total,negative_score_total)
	}

	fn convert_user_comment_score_to_comment_scoring_system(
		user_comment_score: f32,
		comment_scoring_method: CommentScoringMethod,
	) -> f32 {
		match comment_scoring_method {
			CommentScoringMethod::ThumbsUpDown => {
				if user_comment_score < 0.5-THRESHOLD_FOR_VOTING_ON_COMMENT {
					0.0
				} else if user_comment_score > 0.5+THRESHOLD_FOR_VOTING_ON_COMMENT {
					1.0
				} else {
					0.0
				}
			}
			CommentScoringMethod::ZeroToTen => {
				(10.0*user_comment_score).round()
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
				CommentScoringMethod::ThumbsUpDown,
				100,
				LowScoreMemberHandling::Ignore,
				true,
			);

			// Assuming "Top" sorting favors high true quality
			assert!(
				position.0 >= 0.0,
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
				CommentScoringMethod::ThumbsUpDown,
				100,
				LowScoreMemberHandling::Ignore,
				false,
			);

			// Assuming "Top" sorting favors high true quality
			assert!(
				position.0 >= 0.0,
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
				CommentScoringMethod::ThumbsUpDown,
				300,
				LowScoreMemberHandling::Ignore,
				true,
			);

			// Assuming "New" sorting favors recent creation times
			assert!(
				position.0 >= 0.0,
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
				CommentScoringMethod::ThumbsUpDown,
				300,
				LowScoreMemberHandling::Ignore,
				false,
			);

			// Assuming "New" sorting favors recent creation times
			assert!(
				position.0 >= 0.0,
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
				CommentScoringMethod::ThumbsUpDown,
				100,
				LowScoreMemberHandling::Ignore,
				true,
			);

			// Assuming "Best" sorting combines true and perceived quality
			assert!(
				position.0 >= 0.0,
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
				CommentScoringMethod::ThumbsUpDown,
				100,
				LowScoreMemberHandling::Ignore,
				false,
			);
			// Assuming "Best" sorting com
			assert!(
				!position.0.is_nan(),
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
				CommentScoringMethod::ThumbsUpDown,
				100,
				LowScoreMemberHandling::Ignore,
				true,
			);

			// Assuming "Controversial" sorting favors comments with a high variance between true and perceived quality
			assert!(
				position.0 == 7.0,
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
				CommentScoringMethod::ThumbsUpDown,
				100,
				LowScoreMemberHandling::Ignore,
				false,
			);
			// Assuming "Controversial" sorting favors comments with a high variance between true and perceived quality
			assert!(
				position.0 == 0.88888889533,
				"Position should be .333 squared ."
			);
		}

		#[test]
		fn test_comment_compare_operator() {
			let user1 = User {
				id: 1,
//				reputation: 5.0,
				scoring_accuracy: 0.8,
//				preferred_sorting: CommentListSortingMethod::Top,
			};

			let user2 = User {
				id: 2,
//				reputation: 2.0,
				scoring_accuracy: 0.4,
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
