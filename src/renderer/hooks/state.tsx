import { TypedUseSelectorHook, useDispatch, useSelector } from 'react-redux';
import type { AppState, AppDispatch } from '../state/store';

export const useAppDispatch: () => AppDispatch = useDispatch;
export const useAppSelector: TypedUseSelectorHook<AppState> = useSelector;
