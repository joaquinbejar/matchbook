/** Custom error codes the program returns, by name. Engine errors come first. */
export const MatchbookErrorCode = {
  InvalidAccountData: 0,
  AlreadyInitialized: 1,
  InvalidMarketParams: 2,
  OutOfSpace: 3,
  TraderNotFound: 4,
  SeatAlreadyClaimed: 5,
  SeatNotEmpty: 6,
  InvalidPrice: 7,
  InvalidQuantity: 8,
  InsufficientFunds: 9,
  PostOnlyWouldCross: 10,
  FillOrKillNotFilled: 11,
  SelfTrade: 12,
  OrderNotFound: 13,
  Overflow: 14,
  MarketNotActive: 15,
  BudgetRequiresIoc: 16,
  AlreadyExpired: 17,
  Corrupted: 18,
  DelegateNotFound: 19,
  TooManyDelegates: 20,
  NotPermitted: 21,
  DelegateExpired: 22,
  InvalidDelegate: 23,
  InvalidReverseSpread: 24,
  InvalidStop: 25,
  StopWouldTrigger: 26,
  Permissioned: 27,
  InvalidInstruction: 100,
  InvalidInstructionData: 101,
  InvalidAccount: 102,
  MissingSignature: 103,
  InvalidMarketOwner: 104,
  SameMints: 105,
  InvalidMint: 106,
  SlippageExceeded: 107,
  TransferAmountMismatch: 108,
  NotAuthority: 109,
  TooManyItems: 110,
} as const;

/** Name of a MatchBook error code. */
export type MatchbookErrorName = keyof typeof MatchbookErrorCode;

const names = new Map<number, MatchbookErrorName>(
  Object.entries(MatchbookErrorCode).map(([k, v]) => [v, k as MatchbookErrorName]),
);

/** The name of a custom error code, if it is one of MatchBook's. */
export function matchbookErrorName(code: number): MatchbookErrorName | undefined {
  return names.get(code);
}
