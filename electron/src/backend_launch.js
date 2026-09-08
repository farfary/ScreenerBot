'use strict';

/** Pure rollback gate shared by child-process `error` and `exit` events. */
function shouldRollbackStagedCore({ staged, ready, recovering, recoveryScheduled }) {
  return Boolean(staged && !ready && !recovering && !recoveryScheduled);
}

module.exports = { shouldRollbackStagedCore };
