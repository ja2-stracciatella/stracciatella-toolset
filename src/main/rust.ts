// @ts-expect-error Typescript does not recognize native modules and it might not exist yet.
import rustInterface from '../../src-rust/target/stracciatella-toolset.node';

interface RustInterface {
  initLogger: () => void;
  newAppState: () => symbol;
  invoke: (config: symbol, payload: string) => Promise<string>;
}

export default rustInterface as RustInterface;
