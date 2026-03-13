declare module "toml" {
  export function parse(source: string): unknown;
  export function stringify(data: unknown): string;

  const _default: {
    parse: typeof parse;
    stringify: typeof stringify;
  };
  export default _default;
}
