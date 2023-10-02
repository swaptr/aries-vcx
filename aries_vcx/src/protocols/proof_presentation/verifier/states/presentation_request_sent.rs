use aries_vcx_core::{
    anoncreds::base_anoncreds::BaseAnonCreds, ledger::base_ledger::AnoncredsLedgerRead,
};
use messages::msg_fields::protocols::{
    present_proof::{present::Presentation, request::RequestPresentation},
    report_problem::ProblemReport,
};

use crate::{
    common::proofs::verifier::verifier::validate_indy_proof,
    errors::error::{AriesVcxError, AriesVcxErrorKind, VcxResult},
    global::settings,
    handlers::util::{get_attach_as_string, matches_thread_id, Status},
    protocols::proof_presentation::verifier::{
        states::finished::FinishedState, verification_status::PresentationVerificationStatus,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PresentationRequestSentState {
    pub presentation_request: RequestPresentation,
}

impl PresentationRequestSentState {
    pub async fn verify_presentation(
        &self,
        ledger: &impl AnoncredsLedgerRead,
        anoncreds: &impl BaseAnonCreds,
        presentation: &Presentation,
        thread_id: &str,
    ) -> VcxResult<()> {
        if !settings::indy_mocks_enabled() && !matches_thread_id!(presentation, thread_id) {
            return Err(AriesVcxError::from_msg(
                AriesVcxErrorKind::InvalidJson,
                format!(
                    "Cannot handle proof presentation: thread id does not match: {:?}",
                    presentation.decorators.thread.thid
                ),
            ));
        };

        let proof_json = get_attach_as_string!(&presentation.content.presentations_attach);
        let proof_req_json = get_attach_as_string!(
            &self
                .presentation_request
                .content
                .request_presentations_attach
        );

        let valid = validate_indy_proof(ledger, anoncreds, &proof_json, &proof_req_json).await?;

        if !valid {
            return Err(AriesVcxError::from_msg(
                AriesVcxErrorKind::InvalidProof,
                "Presentation verification failed",
            ));
        }

        Ok(())
    }
}

impl
    From<(
        PresentationRequestSentState,
        Presentation,
        PresentationVerificationStatus,
    )> for FinishedState
{
    fn from(
        (state, presentation, verification_status): (
            PresentationRequestSentState,
            Presentation,
            PresentationVerificationStatus,
        ),
    ) -> Self {
        trace!("transit state from PresentationRequestSentState to FinishedState");
        FinishedState {
            presentation_request: Some(state.presentation_request),
            presentation: Some(presentation),
            status: Status::Success,
            verification_status,
        }
    }
}

impl From<(PresentationRequestSentState, ProblemReport)> for FinishedState {
    fn from((state, problem_report): (PresentationRequestSentState, ProblemReport)) -> Self {
        trace!(
            "transit state from PresentationRequestSentState to FinishedState; problem_report: \
             {:?}",
            problem_report
        );
        FinishedState {
            presentation_request: Some(state.presentation_request),
            presentation: None,
            status: Status::Failed(problem_report),
            verification_status: PresentationVerificationStatus::Unavailable,
        }
    }
}
