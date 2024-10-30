mod comment_ranking{
	use rand::{thread_rng, Rng};
	use rand_distr::{Distribution, Normal};
	use std::cmp::Ordering;

	use rand::seq::SliceRandom;
	use rand_distr::num_traits::{pow};
	use rand_distr::Beta;

	#[derive(Clone, Copy)]
	enum SortScoringMethod {
		Linear,
		PlaceSquared
	}
	#[derive(Clone, Copy)]
	enum CommentScoringMethod {
		ThumbsUpDown,
		ZeroToTen
	}

	#[derive(Clone, Copy)]
	enum CommentListSortingMethod {
		Top,
		New,
		Best,
		Controversial
	}


	#[derive(Clone, Copy)]
	enum CommentViewingSort {
		New,
		Hot,
		Top
	}


	#[derive(Copy, Clone)]
	pub struct User {
		id: u16,
		reputation: f32,
		scoring_accuracy: f32,
		preferred_sorting: CommentViewingSort
		//user_comments: Vec<Comment>
	}

	#[derive(Clone, Copy)]
	pub struct AssignedScore {
		score: f32,
		time:u16
	}

	#[derive(Clone)]
	pub struct Comment<'a>{
		true_quality:f32,
		perceived_quality:f32,
		user_scores: Vec<AssignedScore>,// tuple of the score given and when given
		creator: &'a User,
		creation_time: u16

	}

	impl<'a> Comment<'a> {
		fn add_assigned_score(&mut self, score:AssignedScore) {
			self.user_scores.push(score);
		}
	}

	impl Eq for Comment<'_> {
	}
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



	fn simulate_comments_for_one_topic(number_of_comments: u16, number_of_user_interactions:u16, comment_scoring_method: CommentScoringMethod) -> f32{
		let mut rng = thread_rng();
		let users = generate_user_list(number_of_user_interactions);
		//let comment_source_list = generate_one_comments(number_of_comments, &users);
		let mut comment_list_result:Vec< Comment> = vec![];
		for number_of_views_for_topic in 0..number_of_user_interactions {
			let one_user = users.choose(&mut rng).unwrap();
			let should_make_comment = ((number_of_comments / number_of_user_interactions)as f32) < rng.gen();
			if comment_list_result.len()< number_of_comments as usize && (comment_list_result.len() < 1 || should_make_comment ){
				//add comment
				let mut comment = generate_one_comments(&one_user, number_of_views_for_topic);
				comment_list_result.push(comment);
			}
			simulate_one_user_interaction(&mut comment_list_result, one_user, number_of_views_for_topic, comment_scoring_method)
		}
		let mut optimally_sorted_list = comment_list_result.to_vec();

		optimally_sorted_list.sort_unstable();
		let maximum_possible_score = calculate_sorted_comment_list_score(optimally_sorted_list, SortScoringMethod::Linear);

		let score_for_this_sort = calculate_sorted_comment_list_score(comment_list_result, SortScoringMethod::Linear);
		score_for_this_sort/maximum_possible_score
	}

	fn simulate_one_user_interaction(list_of_comments:&mut Vec< Comment>, user: &User, time_of_viewing: u16, comment_scoring_method: CommentScoringMethod) ->(){
		let mut number_of_comments_to_view = thread_rng().sample(Normal::new(3., 1.).unwrap()) as u16;
		let beta = Beta::new(5.0, 5.0).unwrap();
		if number_of_comments_to_view < 0 { number_of_comments_to_view = 0;}
		number_of_comments_to_view += 1; // make sure at least one comment is viewed, assume every user reads at least the top comment
		for comment_index in 0..number_of_comments_to_view{
			let one_comment = list_of_comments.get_mut(comment_index as usize).unwrap();
			let scoring_noise = beta.sample(&mut thread_rng()) +  0.5;// beta goes from 0 to 1, we want this centered around 1
			let new_comment_score = scoring_noise*user.scoring_accuracy*one_comment.true_quality; // multiply how accurate the user is at true quality, and quality.  Then add some noise
			let converted_score = convert_user_comment_score_to_comment_scoring_system(new_comment_score, comment_scoring_method);
			one_comment.add_assigned_score(AssignedScore{score:converted_score, time:time_of_viewing})
		}
	}

	fn generate_one_comments(user: &User, time:u16) -> Comment {
		let mut rng = thread_rng();

		// use a beta distribution rather than normal so all scores are between 0 and 1
		let beta = Beta::new(5.0, 5.0).unwrap();
		let comment_score = beta.sample(&mut rng);
		let one_comment = Comment { true_quality:comment_score, perceived_quality:0.0, user_scores:Vec::new(), creator:user, creation_time:time};
		one_comment
	}

	fn generate_user_list(number_of_users_to_create: u16) -> Vec<User>{
		let mut users = Vec::new();
		let mut rng = thread_rng();
		let beta = Beta::new(5.0, 5.0).unwrap();
		for user_index in 0..number_of_users_to_create{
			let one_user = User{ id:user_index, reputation:beta.sample(&mut rng), scoring_accuracy:beta.sample(&mut rng), preferred_sorting: CommentViewingSort::Hot };
			users.push(one_user);
		}
		users
	}

	fn score_list(comment_list:Vec<Comment>, scoring_method_to_use: CommentListSortingMethod) -> Vec<(f32,Comment)>{
		let mut scored_list = vec![];
		for one_comment in comment_list{
			let score = calculate_sorting_position(&one_comment, scoring_method_to_use);
			scored_list.push((score, one_comment))
		};
		scored_list

	}

	fn calculate_sorted_comment_list_score(sorted_comments: Vec<Comment>, scoring_method: SortScoringMethod) -> f32{
		let mut score = 0.0;
		for comment_index in 0..sorted_comments.len(){
			score +=
				match scoring_method{
					SortScoringMethod::Linear => sorted_comments.get(comment_index).unwrap().true_quality * (sorted_comments.len()-comment_index) as f32,
					SortScoringMethod::PlaceSquared => sorted_comments.get(comment_index).unwrap().true_quality * pow((sorted_comments.len()-comment_index) as f32, 2)
				}
		}
		score
	}

	fn calculate_sorting_position(comment: &Comment, comment_list_sorting_method: CommentListSortingMethod) -> f32{
		match comment_list_sorting_method {
			CommentListSortingMethod::Top => {
				let (positive_scores,negative_scores) = count_positive_and_negative_user_scores(comment);
				(positive_scores - negative_scores) as f32
			}

			CommentListSortingMethod::New => {
				// new means just give the latest comment the highest score
				(u16::MAX - comment.creation_time) as f32
			}

			CommentListSortingMethod::Best => {
				let (positive_scores,negative_scores) = count_positive_and_negative_user_scores(comment);
				let normal_confidence_interval_95_percent = 1.95996398454;
				let number_of_scores  = positive_scores+negative_scores as f32;
				let p_hat = (positive_scores/number_of_scores) as f32;
				let wilson_score_part1 = p_hat+normal_confidence_interval_95_percent.powi(2)/number_of_scores;
				let wilson_score_under_radical = (p_hat*(1.0-p_hat) + normal_confidence_interval_95_percent.powi(2)/(4*number_of_scores))/number_of_scores;
				let wilson_score_part2 = normal_confidence_interval_95_percent*wilson_score_under_radical.sqrt();
				let wilson_score_lower_bound = (wilson_score_part1-wilson_score_part2)/(1+normal_confidence_interval_95_percent.powi(2)/number_of_scores);
				wilson_score_lower_bound
			}

			CommentListSortingMethod::Controversial => {
				let (positive_scores,negative_scores) = count_positive_and_negative_user_scores(comment);
				// not sure how it's done in reddit, but here we'll use how balanced total votes are
				1-pow((positive_scores - negative_scores)/(positive_scores+negative_scores), 2) as f32

			}
		}
	}

	fn count_positive_and_negative_user_scores(comment: &Comment)->(u16,u16){
		let mut positive_scores  = 0;
		let mut negative_scores = 0;
		for one_user_score in comment.user_scores{
			if one_user_score.score > 0 as f32 {
				positive_scores += 1;
			}else if one_user_score.score < 0 as f32 {
				negative_scores += 1;
			}
		}
		(positive_scores,negative_scores)

	}

	fn convert_user_comment_score_to_comment_scoring_system(user_comment_score:f32,comment_scoring_method: CommentScoringMethod)->f32{
		match comment_scoring_method {
			CommentScoringMethod::ThumbsUpDown => {
				if user_comment_score< -0.5 {
					-1 as f32
				}else if user_comment_score> 0.5 {
					1 as f32
				}else {
					0.0
				}
			}
			CommentScoringMethod::ZeroToTen => {
				if user_comment_score<0.0 {
					5.0*(1.0-(user_comment_score/(user_comment_score-1.0)))
				} else{
					5.0*(user_comment_score/(user_comment_score+1.0)) + 5.0
				}
			}
		}
	}

	fn generate_optimally_sorted_comments(comments: Vec<Comment>) -> Vec<Comment>{
		let mut sorted_comments =comments.to_vec();
		sorted_comments.sort_unstable();
		sorted_comments
	}
}
