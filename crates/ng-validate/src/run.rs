use ng_contract::SimTime;
use ng_record::{Faulted, Running, Session, Step};

pub(crate) fn advance<'m>(
    session: Session<'m, Running>,
) -> Result<Session<'m, Running>, Session<'m, Faulted>> {
    let mut attempt = session.step();
    loop {
        match attempt {
            Step::Accepted(session, _) => return Ok(session),
            Step::Refused(session) => attempt = session.retry(),
            Step::Faulted(session) => return Err(session),
        }
    }
}

/// Steps until the accepted head reaches `end`, retrying every refusal. A fault ends it with the latch.
pub fn run_until<'m>(
    mut session: Session<'m, Running>,
    end: SimTime,
) -> Result<Session<'m, Running>, Session<'m, Faulted>> {
    while session.record().accepted().time() < end {
        session = advance(session)?;
    }
    Ok(session)
}
