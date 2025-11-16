import { Flex } from 'antd';
import { Loader } from './Loader';

interface FullSizeLoaderProps {
  size?: 'small' | 'default' | 'large';
}

const FLEX_STYLE = {
  width: '100%',
  height: '100%',
};

export function FullSizeLoader({ size = 'large' }: FullSizeLoaderProps) {
  return (
    <Flex align="center" justify="center" style={FLEX_STYLE}>
      <Loader size={size} />
    </Flex>
  );
}
