'use strict';

const { StringDecoder } = require('string_decoder');

/** Pure first-activation rollback gate shared by child `error` and `exit`. */
function shouldRollbackStagedCore({ staged, firstRun, ready, recovering, recoveryScheduled }) {
  return Boolean(staged && firstRun && !ready && !recovering && !recoveryScheduled);
}

/** Preserve complete lines across arbitrary stdout/stderr chunk boundaries. */
function createLineDecoder(onLine) {
  const decoder = new StringDecoder('utf8');
  let pending = '';

  const drain = () => {
    let newline;
    while ((newline = pending.indexOf('\n')) !== -1) {
      const line = pending.slice(0, newline).replace(/\r$/, '');
      pending = pending.slice(newline + 1);
      onLine(line);
    }
  };

  return {
    push(chunk) {
      pending += decoder.write(chunk);
      drain();
    },
    end(chunk) {
      pending += decoder.end(chunk);
      drain();
      if (pending) onLine(pending.replace(/\r$/, ''));
      pending = '';
    },
  };
}

module.exports = { createLineDecoder, shouldRollbackStagedCore };
