import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { RangeSlider } from './RangeSlider';

describe('RangeSlider Component', () => {
  it('se rend correctement avec label, displayValue, minLabel et maxLabel', () => {
    const onChange = vi.fn();
    render(
      <RangeSlider
        id="test-slider"
        min={0}
        max={100}
        value={50}
        onChange={onChange}
        label="Slider Test"
        displayValue="50%"
        minLabel="Min 0"
        maxLabel="Max 100"
        helpText="Aide contextuelle"
      />
    );

    expect(screen.getByLabelText('Slider Test')).toBeInTheDocument();
    expect(screen.getByText('50%')).toBeInTheDocument();
    expect(screen.getByText('Min 0')).toBeInTheDocument();
    expect(screen.getByText('Max 100')).toBeInTheDocument();
    expect(screen.getByText('Aide contextuelle')).toBeInTheDocument();
  });

  it('appelle onChange lorsque la valeur change', () => {
    const onChange = vi.fn();
    render(
      <RangeSlider
        id="test-slider"
        min={0}
        max={100}
        value={50}
        onChange={onChange}
      />
    );

    const input = screen.getByRole('slider');
    fireEvent.change(input, { target: { value: '75' } });
    expect(onChange).toHaveBeenCalledWith(75);
  });
});
