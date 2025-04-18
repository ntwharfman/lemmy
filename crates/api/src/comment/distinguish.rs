use activitypub_federation::config::Data;
use actix_web::web::Json;
use lemmy_api_common::{
  comment::{CommentResponse, DistinguishComment},
  context::LemmyContext,
  send_activity::{ActivityChannel, SendActivityData},
  utils::{check_community_mod_action, check_community_user_action},
};
use lemmy_db_schema::{
  source::comment::{Comment, CommentUpdateForm},
  traits::Crud,
};
use lemmy_db_views::structs::{CommentView, LocalUserView};
use lemmy_utils::error::{LemmyErrorExt, LemmyErrorType, LemmyResult};

pub async fn distinguish_comment(
  data: Json<DistinguishComment>,
  context: Data<LemmyContext>,
  local_user_view: LocalUserView,
) -> LemmyResult<Json<CommentResponse>> {
  let orig_comment = CommentView::read(
    &mut context.pool(),
    data.comment_id,
    Some(&local_user_view.local_user),
  )
  .await?;

  check_community_user_action(
    &local_user_view,
    &orig_comment.community,
    &mut context.pool(),
  )
  .await?;

  // Verify that only the creator can distinguish
  if local_user_view.person.id != orig_comment.creator.id {
    Err(LemmyErrorType::NoCommentEditAllowed)?
  }

  // Verify that only a mod or admin can distinguish a comment
  check_community_mod_action(
    &local_user_view,
    &orig_comment.community,
    false,
    &mut context.pool(),
  )
  .await?;

  // Update the Comment
  let form = CommentUpdateForm {
    distinguished: Some(data.distinguished),
    ..Default::default()
  };
  let comment = Comment::update(&mut context.pool(), data.comment_id, &form)
    .await
    .with_lemmy_type(LemmyErrorType::CouldntUpdateComment)?;

  ActivityChannel::submit_activity(SendActivityData::UpdateComment(comment), &context)?;

  let comment_view = CommentView::read(
    &mut context.pool(),
    data.comment_id,
    Some(&local_user_view.local_user),
  )
  .await?;

  Ok(Json(CommentResponse {
    comment_view,
    recipient_ids: Vec::new(),
  }))
}
