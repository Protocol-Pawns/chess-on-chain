import {
  AccessKeyDoesNotExistError,
  AccessKeyNotEnoughAllowanceActionError,
  AccessKeyNotFoundActionError
} from 'near-api-js/rpc-errors';

function message(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}

export function isAccessKeyError(err: unknown): boolean {
  return (
    err instanceof AccessKeyNotEnoughAllowanceActionError ||
    err instanceof AccessKeyNotFoundActionError ||
    err instanceof AccessKeyDoesNotExistError ||
    /not enough allowance/i.test(message(err)) ||
    /access key.*does not exist/i.test(message(err)) ||
    /access key not found/i.test(message(err))
  );
}
