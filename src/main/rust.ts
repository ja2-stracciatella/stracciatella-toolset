// eslint-disable-next-line import/no-unresolved, @typescript-eslint/ban-ts-comment
// @ts-ignore
import rustInterface from '../../src-rust/target/stracciatella-toolset.node';

interface RustInterface {
  initLogger: () => void;
  newAppState: () => Symbol;
  invoke: (config: Symbol, payload: string) => Promise<string>;
}

export default rustInterface as RustInterface;
