use std::collections::{HashMap, HashSet, VecDeque};
use std::mem;
use std::time::{Duration, Instant};

use super::{CommissionAndFeesReport, ExecutionData, Executions};

/// An execution with its commission report, joined by `execution_id`.
///
/// Built by [`FillMatcher`].
///
/// # Examples
///
/// ```
/// use ibapi::orders::{CommissionAndFeesReport, Execution, ExecutionData, Fill};
///
/// let fill = Fill {
///     execution: ExecutionData {
///         execution: Execution { execution_id: "0000e1a7.0001.01".to_string(), shares: 100.0, ..Default::default() },
///         ..Default::default()
///     },
///     commission: Some(CommissionAndFeesReport { execution_id: "0000e1a7.0001.01".to_string(), commission_and_fees: 1.25, ..Default::default() }),
/// };
/// let fees = fill.commission.as_ref().map(|report| report.commission_and_fees);
/// println!("{} shares, commissions and fees {fees:?}", fill.execution.execution.shares);
/// ```
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct Fill {
    /// The execution and its contract.
    pub execution: ExecutionData,
    /// The execution's commission report. `None` when the matcher's rule says TWS sends none
    /// for this execution (see [`FillMatcher::expects_report`]), or when the execution came
    /// from [`FillMatcher::take_expired`].
    pub commission: Option<CommissionAndFeesReport>,
}

/// Executions and commission reports that found no partner, in arrival order.
///
/// Returned by [`FillMatcher::take_expired`] and [`FillMatcher::pair_executions`].
///
/// # Examples
///
/// ```
/// use ibapi::orders::{Executions, FillMatcher};
///
/// let rows: Vec<Executions> = Vec::new(); // an executions() reply, read to its end
/// let (_fills, unpaired) = FillMatcher::new().pair_executions(rows);
/// for execution in &unpaired.executions {
///     println!("no commission report for {}", execution.execution.execution_id);
/// }
/// for report in &unpaired.reports {
///     println!("no execution for report {}", report.execution_id);
/// }
/// ```
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug, Default)]
pub struct Unpaired {
    /// Executions whose commission report did not arrive. From
    /// [`FillMatcher::pair_executions`], usually a fill so recent that TWS had not sent its
    /// report yet: a reply that lands between a live execution and its report lists the
    /// execution without one, and the report follows on the live stream after the reply's end
    /// (live 2026-11-24, paper TWS, server 223). Read the reply again, or wait for the report
    /// on the stream, before treating the commission as missing.
    pub executions: Vec<ExecutionData>,
    /// Commission reports whose execution did not arrive.
    pub reports: Vec<CommissionAndFeesReport>,
}

/// What [`FillMatcher::push_report`] did with a commission report.
///
/// # Examples
///
/// ```
/// use ibapi::orders::{CommissionAndFeesReport, FillMatcher, ReportMatch};
///
/// let mut matcher = FillMatcher::new();
/// let report = CommissionAndFeesReport { execution_id: "0000e1a7.0001.01".to_string(), ..Default::default() };
/// match matcher.push_report(report) {
///     ReportMatch::Paired(fill) => println!("fill: {fill:?}"),
///     ReportMatch::Waiting => println!("execution not seen yet"),
///     ReportMatch::Late(report) => println!("execution already handed out without it: {report:?}"),
///     ReportMatch::Duplicate(report) => println!("already seen: {report:?}"),
/// }
/// ```
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ReportMatch {
    /// The execution was waiting; here is the joined fill.
    Paired(Fill),
    /// The execution has not arrived; the matcher holds the report until it does or
    /// [`FillMatcher::take_expired`] returns it.
    Waiting,
    /// The first report for an execution the matcher handed out without a commission, by
    /// [`FillMatcher::take_expired`] or because its rule expects no report. The matcher
    /// remembers that it has now seen a report for the execution.
    Late(CommissionAndFeesReport),
    /// A report for an execution that already has one: paired, or answered as `Late` before.
    /// Also a second copy of a report that is still waiting. The matcher does not keep it.
    Duplicate(CommissionAndFeesReport),
}

/// Whether an execution the matcher handed out has had a report.
#[derive(Clone, Copy, Debug)]
enum Handout {
    Unreported { arrived: Instant },
    Reported,
}

/// Joins each [`ExecutionData`] with its [`CommissionAndFeesReport`] as both arrive.
///
/// TWS sends an execution and its commission report as two messages that share only
/// `execution_id`, with other frames between them. Feed the matcher every `ExecutionData` and
/// `CommissionAndFeesReport` from [`OrderUpdate`](super::OrderUpdate),
/// [`PlaceOrder`](super::PlaceOrder) or [`Executions`] items, in either order: it returns the
/// [`Fill`] once both halves are known. For a whole [`executions`](crate::Client::executions)
/// reply, use [`pair_executions`](Self::pair_executions).
///
/// Some executions never get a report. The matcher returns those at once with no commission,
/// as its rule says (by default [`ExecutionData::expects_commission_report`]; set your own with
/// [`expects_report`](Self::expects_report)). For any other execution whose report does not
/// come, call [`take_expired`](Self::take_expired) periodically.
///
/// The matcher drops copies of executions it holds and copies whose `execution_id` remains in
/// its bounded handout memory. The same execution can arrive live on
/// [`order_update_stream`](crate::Client::order_update_stream) and again in an
/// [`executions`](crate::Client::executions) reply.
///
/// # What it holds
///
/// - Executions waiting for their report, and reports that came before their execution, until
///   they pair or `take_expired` returns them. One `max_wait` bounds both: a report precedes
///   its execution by milliseconds when it does at all, and a repeated report never waits
///   (see below).
/// - The ids of the executions it handed out, the last [`memory`](Self::memory) of them
///   ([`DEFAULT_MEMORY`](Self::DEFAULT_MEMORY) by default), oldest dropped first. Another copy
///   of one of these executions is dropped. A report for one never waits: it is
///   [`ReportMatch::Late`] the first time for an execution handed out without a commission,
///   and [`ReportMatch::Duplicate`] after that and for a paired execution.
///
/// It is plain data: no I/O, no tasks, the same from sync and async code. Each method that
/// reads the clock has an `_at` twin that takes the time instead, for deterministic tests.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "async")]
/// # async fn run(client: &ibapi::Client) -> Result<(), ibapi::Error> {
/// use futures::StreamExt;
/// use ibapi::orders::{FillMatcher, OrderUpdate, ReportMatch};
/// use ibapi::subscriptions::SubscriptionItem;
/// use std::time::Duration;
///
/// let mut stream = client.order_update_stream().await?;
/// let mut matcher = FillMatcher::new();
/// loop {
///     let item = match tokio::time::timeout(Duration::from_secs(1), stream.next()).await {
///         Ok(Some(item)) => item,
///         Ok(None) => break,
///         Err(_elapsed) => {
///             for execution in matcher.take_expired(Duration::from_secs(5)).executions {
///                 println!("fill without commission: {execution:?}");
///             }
///             continue;
///         }
///     };
///     match item? {
///         SubscriptionItem::Data(OrderUpdate::ExecutionData(execution)) => {
///             if let Some(fill) = matcher.push_execution(execution) {
///                 println!("fill: {fill:?}");
///             }
///         }
///         SubscriptionItem::Data(OrderUpdate::CommissionAndFeesReport(report)) => match matcher.push_report(report) {
///             ReportMatch::Paired(fill) => println!("fill: {fill:?}"),
///             ReportMatch::Late(report) => println!("late commission: {report:?}"),
///             ReportMatch::Waiting | ReportMatch::Duplicate(_) => {}
///         },
///         SubscriptionItem::Data(update) => println!("update: {update:?}"),
///         SubscriptionItem::Notice(notice) => println!("notice: {}", notice.message),
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct FillMatcher {
    expects_report: fn(&ExecutionData) -> bool,
    memory: usize,
    executions: Vec<(ExecutionData, Instant)>,
    reports: Vec<(CommissionAndFeesReport, Instant)>,
    handouts: HashMap<String, (u64, Handout)>,
    handout_order: VecDeque<(u64, String)>,
    handout_count: u64,
}

impl Default for FillMatcher {
    fn default() -> Self {
        Self {
            expects_report: ExecutionData::expects_commission_report,
            memory: Self::DEFAULT_MEMORY,
            executions: Vec::new(),
            reports: Vec::new(),
            handouts: HashMap::new(),
            handout_order: VecDeque::new(),
            handout_count: 0,
        }
    }
}

impl FillMatcher {
    /// How many handed-out execution ids a matcher remembers unless
    /// [`memory`](Self::memory) says otherwise.
    pub const DEFAULT_MEMORY: usize = 10_000;

    /// Creates an empty matcher with the default rule and memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::FillMatcher;
    /// use std::time::Duration;
    ///
    /// let mut matcher = FillMatcher::new();
    /// let unpaired = matcher.take_expired(Duration::ZERO);
    /// assert!(unpaired.executions.is_empty() && unpaired.reports.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets how many handed-out execution ids the matcher remembers to recognize a late or
    /// repeated report, or a repeated execution; the oldest is dropped first. A report or
    /// execution for a dropped id is treated as new. `0` remembers none.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::FillMatcher;
    ///
    /// let matcher = FillMatcher::new().memory(50_000);
    /// ```
    pub fn memory(mut self, capacity: usize) -> Self {
        self.memory = capacity;
        self.evict();
        self
    }

    /// Sets the rule that says whether TWS sends a report for an execution. An execution it
    /// returns `false` for is a [`Fill`] with no commission as soon as it arrives, unless its
    /// report came first. The default is [`ExecutionData::expects_commission_report`], which
    /// covers only what was verified live.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{ExecutionData, FillMatcher, OptionExerciseType};
    ///
    /// // Also expect no report for any exercise type.
    /// let matcher = FillMatcher::new().expects_report(|data: &ExecutionData| {
    ///     data.expects_commission_report() && data.execution.option_exercise_type == OptionExerciseType::None
    /// });
    /// ```
    pub fn expects_report(mut self, rule: fn(&ExecutionData) -> bool) -> Self {
        self.expects_report = rule;
        self
    }

    /// Adds an execution. Returns the [`Fill`] when its report has already arrived, or when
    /// the rule expects no report (then with `commission: None`); otherwise holds the
    /// execution and returns `None`. Also returns `None`, and drops the execution, when the
    /// matcher still holds or remembers an execution with its `execution_id`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{CommissionAndFeesReport, Execution, ExecutionData, FillMatcher, ReportMatch};
    ///
    /// let mut matcher = FillMatcher::new();
    /// let report = CommissionAndFeesReport { execution_id: "0000e1a7.0001.01".to_string(), commission_and_fees: 1.25, ..Default::default() };
    /// assert!(matches!(matcher.push_report(report), ReportMatch::Waiting));
    ///
    /// let execution = ExecutionData {
    ///     execution: Execution { execution_id: "0000e1a7.0001.01".to_string(), ..Default::default() },
    ///     ..Default::default()
    /// };
    /// let fill = matcher.push_execution(execution).expect("report arrived first");
    /// assert_eq!(fill.commission.map(|report| report.commission_and_fees), Some(1.25));
    /// ```
    pub fn push_execution(&mut self, execution: ExecutionData) -> Option<Fill> {
        self.push_execution_at(execution, Instant::now())
    }

    /// [`push_execution`](Self::push_execution) with the arrival time given rather than read
    /// from the clock.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{Execution, ExecutionData, FillMatcher};
    /// use std::time::{Duration, Instant};
    ///
    /// let start = Instant::now();
    /// let mut matcher = FillMatcher::new();
    /// let execution = ExecutionData {
    ///     execution: Execution { execution_id: "0000e1a7.0001.01".to_string(), ..Default::default() },
    ///     ..Default::default()
    /// };
    /// assert!(matcher.push_execution_at(execution, start).is_none());
    /// assert_eq!(matcher.take_expired_at(Duration::from_secs(5), start + Duration::from_secs(5)).executions.len(), 1);
    /// ```
    pub fn push_execution_at(&mut self, execution: ExecutionData, now: Instant) -> Option<Fill> {
        let execution_id = &execution.execution.execution_id;
        if self.handouts.contains_key(execution_id) || self.executions.iter().any(|(held, _)| held.execution.execution_id == *execution_id) {
            return None;
        }
        if let Some(index) = self.reports.iter().position(|(report, _)| report.execution_id == *execution_id) {
            let (report, _) = self.reports.remove(index);
            self.remember(execution_id.clone(), Handout::Reported);
            return Some(Fill {
                execution,
                commission: Some(report),
            });
        }
        if !(self.expects_report)(&execution) {
            self.remember(execution_id.clone(), Handout::Unreported { arrived: now });
            return Some(Fill { execution, commission: None });
        }
        self.executions.push((execution, now));
        None
    }

    /// Adds a commission report: pairs it with its waiting execution, recognizes it as late or
    /// repeated, or holds it until its execution arrives. See [`ReportMatch`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{CommissionAndFeesReport, Execution, ExecutionData, FillMatcher, ReportMatch};
    ///
    /// let mut matcher = FillMatcher::new();
    /// let execution = ExecutionData {
    ///     execution: Execution { execution_id: "0000e1a7.0001.01".to_string(), ..Default::default() },
    ///     ..Default::default()
    /// };
    /// assert!(matcher.push_execution(execution).is_none());
    ///
    /// let report = CommissionAndFeesReport { execution_id: "0000e1a7.0001.01".to_string(), commission_and_fees: 1.25, ..Default::default() };
    /// match matcher.push_report(report.clone()) {
    ///     ReportMatch::Paired(fill) => println!("{} paid {:?}", fill.execution.execution.execution_id, fill.commission),
    ///     other => panic!("expected a fill, got {other:?}"),
    /// }
    /// assert!(matches!(matcher.push_report(report), ReportMatch::Duplicate(_)));
    /// ```
    pub fn push_report(&mut self, report: CommissionAndFeesReport) -> ReportMatch {
        self.push_report_at(report, Instant::now())
    }

    /// [`push_report`](Self::push_report) with the arrival time given rather than read from
    /// the clock.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{CommissionAndFeesReport, FillMatcher, ReportMatch};
    /// use std::time::{Duration, Instant};
    ///
    /// let start = Instant::now();
    /// let mut matcher = FillMatcher::new();
    /// let report = CommissionAndFeesReport { execution_id: "0000e1a7.0001.01".to_string(), ..Default::default() };
    /// assert!(matches!(matcher.push_report_at(report, start), ReportMatch::Waiting));
    /// assert_eq!(matcher.take_expired_at(Duration::from_secs(5), start + Duration::from_secs(5)).reports.len(), 1);
    /// ```
    pub fn push_report_at(&mut self, report: CommissionAndFeesReport, now: Instant) -> ReportMatch {
        if let Some(index) = self
            .executions
            .iter()
            .position(|(execution, _)| execution.execution.execution_id == report.execution_id)
        {
            let (execution, _) = self.executions.remove(index);
            self.remember(report.execution_id.clone(), Handout::Reported);
            return ReportMatch::Paired(Fill {
                execution,
                commission: Some(report),
            });
        }
        if let Some((_, handout)) = self.handouts.get_mut(&report.execution_id) {
            return match mem::replace(handout, Handout::Reported) {
                Handout::Unreported { .. } => ReportMatch::Late(report),
                Handout::Reported => ReportMatch::Duplicate(report),
            };
        }
        if self.reports.iter().any(|(waiting, _)| waiting.execution_id == report.execution_id) {
            return ReportMatch::Duplicate(report);
        }
        self.reports.push((report, now));
        ReportMatch::Waiting
    }

    /// Removes and returns the executions and reports that have waited at least `max_wait`
    /// for their partner. The first report for an execution whose ID remains in
    /// [`memory`](Self::memory) is [`ReportMatch::Late`]. `Duration::ZERO` returns everything
    /// the matcher holds.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{Execution, ExecutionData, Fill, FillMatcher};
    /// use std::time::Duration;
    ///
    /// let mut matcher = FillMatcher::new();
    /// let execution = ExecutionData {
    ///     execution: Execution { execution_id: "0000e1a7.0001.01".to_string(), ..Default::default() },
    ///     ..Default::default()
    /// };
    /// assert!(matcher.push_execution(execution).is_none());
    ///
    /// // No report came: emit the execution without a commission.
    /// for execution in matcher.take_expired(Duration::ZERO).executions {
    ///     let fill = Fill { execution, commission: None };
    ///     println!("fill without commission: {fill:?}");
    /// }
    /// ```
    pub fn take_expired(&mut self, max_wait: Duration) -> Unpaired {
        self.take_expired_at(max_wait, Instant::now())
    }

    /// [`take_expired`](Self::take_expired) as of `now` rather than the clock.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{Execution, ExecutionData, FillMatcher};
    /// use std::time::{Duration, Instant};
    ///
    /// let start = Instant::now();
    /// let mut matcher = FillMatcher::new();
    /// let execution = ExecutionData {
    ///     execution: Execution { execution_id: "0000e1a7.0001.01".to_string(), ..Default::default() },
    ///     ..Default::default()
    /// };
    /// matcher.push_execution_at(execution, start);
    ///
    /// let max_wait = Duration::from_secs(5);
    /// assert!(matcher.take_expired_at(max_wait, start + Duration::from_secs(4)).executions.is_empty());
    /// assert_eq!(matcher.take_expired_at(max_wait, start + max_wait).executions.len(), 1);
    /// ```
    pub fn take_expired_at(&mut self, max_wait: Duration, now: Instant) -> Unpaired {
        let expired = |at: &Instant| now.saturating_duration_since(*at) >= max_wait;
        self.take_where(|_, at| expired(at), |_, at| expired(at))
    }

    /// Returns an execution that [`take_expired`](Self::take_expired) handed out to the
    /// matcher, to wait for its report again from its original arrival time, as if it had
    /// never been taken. For a consumer that took an execution to emit it without a
    /// commission and cannot emit it yet.
    ///
    /// Gives the execution back, unchanged, when the matcher has no record of handing it out
    /// without a report: a report for it has come since (as [`ReportMatch::Late`]; pair the
    /// two yourself), [`memory`](Self::memory) dropped the record, or the matcher never
    /// handed it out.
    ///
    /// # Examples
    ///
    /// ```
    /// use ibapi::orders::{CommissionAndFeesReport, Execution, ExecutionData, FillMatcher, ReportMatch};
    /// use std::time::Duration;
    ///
    /// let mut matcher = FillMatcher::new();
    /// let execution = ExecutionData {
    ///     execution: Execution { execution_id: "0000e1a7.0001.01".to_string(), ..Default::default() },
    ///     ..Default::default()
    /// };
    /// matcher.push_execution(execution);
    ///
    /// for execution in matcher.take_expired(Duration::ZERO).executions {
    ///     // Not ready to emit it: wait for its report a while longer.
    ///     assert!(matcher.put_back(execution).is_none());
    /// }
    /// let report = CommissionAndFeesReport { execution_id: "0000e1a7.0001.01".to_string(), ..Default::default() };
    /// assert!(matches!(matcher.push_report(report), ReportMatch::Paired(_)));
    /// ```
    pub fn put_back(&mut self, execution: ExecutionData) -> Option<ExecutionData> {
        let execution_id = &execution.execution.execution_id;
        let Some(&(_, Handout::Unreported { arrived })) = self.handouts.get(execution_id) else {
            return Some(execution);
        };
        self.handouts.remove(execution_id);
        let index = self.executions.partition_point(|(_, at)| *at <= arrived);
        self.executions.insert(index, (execution, arrived));
        None
    }

    /// Pairs the executions and commission reports of an
    /// [`executions`](crate::Client::executions) reply. Returns the fills, in the order their
    /// executions appear in the reply, and the executions and reports of the reply that found
    /// no partner. A report that comes after an execution the rule expects none for fills in
    /// that fill's commission; a repeated report is dropped. A first report for an execution
    /// the matcher handed out without one before the call is returned in
    /// [`Unpaired::reports`], after the reports that found no execution.
    ///
    /// Pass the whole reply: read it with `collect_to_end_within`, which fails rather than
    /// return a partial reply, so an execution whose report was cut off never shows up as
    /// unpaired. While an execution stays in [`memory`](Self::memory), the matcher drops its
    /// copies and classifies a later report as [`ReportMatch::Duplicate`] or, for the first
    /// report of an unpaired execution, [`ReportMatch::Late`].
    ///
    /// A reply item can pair with an execution or report already waiting from a live stream.
    /// A copy of a waiting item leaves the original waiting for its counterpart. An unpaired
    /// execution is most often a fill whose report TWS has not sent yet. See
    /// [`Unpaired::executions`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run(client: &ibapi::Client) -> Result<(), ibapi::Error> {
    /// use ibapi::orders::{ExecutionFilter, FillMatcher};
    /// use std::time::Duration;
    ///
    /// let mut subscription = client.executions(ExecutionFilter::default()).await?;
    /// let rows = subscription.collect_to_end_within(Duration::from_secs(10)).await?;
    ///
    /// let mut matcher = FillMatcher::new();
    /// let (fills, unpaired) = matcher.pair_executions(rows);
    /// for fill in &fills {
    ///     let fees = fill.commission.as_ref().map(|report| report.commission_and_fees);
    ///     println!("{} {} @ {}: {fees:?}", fill.execution.contract.symbol, fill.execution.execution.shares, fill.execution.execution.price);
    /// }
    /// println!("{} without a report, {} reports without an execution", unpaired.executions.len(), unpaired.reports.len());
    /// // Keep feeding `matcher` from order_update_stream to recognize copies while their IDs are retained.
    /// # Ok(())
    /// # }
    /// ```
    pub fn pair_executions(&mut self, rows: impl IntoIterator<Item = Executions>) -> (Vec<Fill>, Unpaired) {
        let now = Instant::now();
        let held: HashSet<String> = self
            .executions
            .iter()
            .map(|(execution, _)| execution.execution.execution_id.clone())
            .collect();
        let mut positions: HashMap<String, usize> = HashMap::new();
        let mut report_ids: HashSet<String> = HashSet::new();
        let mut fills: Vec<Fill> = Vec::new();
        let mut late: Vec<CommissionAndFeesReport> = Vec::new();
        for row in rows {
            match row {
                Executions::ExecutionData(execution) => {
                    let position = positions.len();
                    positions.entry(execution.execution.execution_id.clone()).or_insert(position);
                    fills.extend(self.push_execution_at(execution, now));
                }
                Executions::CommissionAndFeesReport(report) => {
                    let execution_id = report.execution_id.clone();
                    match self.push_report_at(report, now) {
                        ReportMatch::Paired(fill) => fills.push(fill),
                        ReportMatch::Late(report) => match fills.iter_mut().find(|fill| fill.execution.execution.execution_id == report.execution_id)
                        {
                            Some(fill) => fill.commission = Some(report),
                            None => late.push(report),
                        },
                        ReportMatch::Waiting => {
                            report_ids.insert(execution_id);
                        }
                        ReportMatch::Duplicate(_) => {}
                    }
                }
            }
        }
        fills.sort_by_key(|fill| positions.get(&fill.execution.execution.execution_id).copied().unwrap_or(usize::MAX));
        let mut unpaired = self.take_where(
            |execution, _| {
                let execution_id = &execution.execution.execution_id;
                positions.contains_key(execution_id) && !held.contains(execution_id)
            },
            |report, _| report_ids.contains(&report.execution_id),
        );
        unpaired.reports.extend(late);
        (fills, unpaired)
    }

    fn take_where(
        &mut self,
        mut take_execution: impl FnMut(&ExecutionData, &Instant) -> bool,
        mut take_report: impl FnMut(&CommissionAndFeesReport, &Instant) -> bool,
    ) -> Unpaired {
        let (taken, kept): (Vec<_>, Vec<_>) = mem::take(&mut self.executions)
            .into_iter()
            .partition(|(execution, at)| take_execution(execution, at));
        self.executions = kept;
        let (taken_reports, kept_reports): (Vec<_>, Vec<_>) =
            mem::take(&mut self.reports).into_iter().partition(|(report, at)| take_report(report, at));
        self.reports = kept_reports;

        for (execution, arrived) in &taken {
            self.remember(execution.execution.execution_id.clone(), Handout::Unreported { arrived: *arrived });
        }
        Unpaired {
            executions: taken.into_iter().map(|(execution, _)| execution).collect(),
            reports: taken_reports.into_iter().map(|(report, _)| report).collect(),
        }
    }

    fn remember(&mut self, execution_id: String, handout: Handout) {
        if self.memory == 0 {
            return;
        }
        self.handout_count += 1;
        let sequence = self.handout_count;
        self.handout_order.push_back((sequence, execution_id.clone()));
        self.handouts.insert(execution_id, (sequence, handout));
        self.evict();
    }

    fn evict(&mut self) {
        while self.handouts.len() > self.memory {
            let Some((sequence, execution_id)) = self.handout_order.pop_front() else {
                break;
            };
            if self.handouts.get(&execution_id).is_some_and(|(current, _)| *current == sequence) {
                self.handouts.remove(&execution_id);
            }
        }
        if self.handout_order.len() > self.memory.saturating_mul(2) {
            let handouts = &self.handouts;
            self.handout_order
                .retain(|(sequence, execution_id)| handouts.get(execution_id).is_some_and(|(current, _)| current == sequence));
        }
    }
}

#[cfg(test)]
#[path = "fills_tests.rs"]
mod tests;
