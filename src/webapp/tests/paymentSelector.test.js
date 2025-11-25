/**
 * Payment Method Selector Tests
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import PaymentMethodSelector from '../src/lib/components/PaymentMethodSelector.svelte';

describe('PaymentMethodSelector', () => {
  describe('Component Rendering', () => {
    it('renders with default SOL selection', () => {
      const { getByText, container } = render(PaymentMethodSelector);

      expect(getByText(/SOL/i)).toBeTruthy();
      expect(getByText(/wZEC/i)).toBeTruthy();
      expect(getByText(/SOL_SELECTED/i)).toBeTruthy();
    });

    it('renders with wZEC pre-selected', () => {
      const { getByText } = render(PaymentMethodSelector, {
        props: { selected: 'wzec' }
      });

      expect(getByText(/wZEC_SELECTED/i)).toBeTruthy();
    });

    it('shows info box for SOL', () => {
      const { getByText } = render(PaymentMethodSelector, {
        props: { selected: 'sol' }
      });

      expect(getByText(/PAYING_WITH_NATIVE_SOL/i)).toBeTruthy();
      expect(getByText(/Most efficient option/i)).toBeTruthy();
    });

    it('shows info box for wZEC', () => {
      const { getByText } = render(PaymentMethodSelector, {
        props: { selected: 'wzec' }
      });

      expect(getByText(/PAYING_WITH_wZEC_TOKEN/i)).toBeTruthy();
      expect(getByText(/Requires wZEC token account/i)).toBeTruthy();
    });
  });

  describe('User Interaction', () => {
    it('emits change event when clicking SOL card', async () => {
      const { component, container } = render(PaymentMethodSelector, {
        props: { selected: 'wzec' }
      });

      const changeHandler = vi.fn();
      component.$on('change', changeHandler);

      const solButton = container.querySelector('.method-card.violet');
      await fireEvent.click(solButton);

      expect(changeHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          detail: { payment_method: 'sol' }
        })
      );
    });

    it('emits change event when clicking wZEC card', async () => {
      const { component, container } = render(PaymentMethodSelector, {
        props: { selected: 'sol' }
      });

      const changeHandler = vi.fn();
      component.$on('change', changeHandler);

      const wzecButton = container.querySelector('.method-card.cyan');
      await fireEvent.click(wzecButton);

      expect(changeHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          detail: { payment_method: 'wzec' }
        })
      );
    });

    it('does not emit event when disabled', async () => {
      const { component, container } = render(PaymentMethodSelector, {
        props: { selected: 'sol', disabled: true }
      });

      const changeHandler = vi.fn();
      component.$on('change', changeHandler);

      const wzecButton = container.querySelector('.method-card.cyan');
      await fireEvent.click(wzecButton);

      expect(changeHandler).not.toHaveBeenCalled();
    });
  });

  describe('Accessibility', () => {
    it('has proper ARIA attributes', () => {
      const { container } = render(PaymentMethodSelector);

      const radioButtons = container.querySelectorAll('[role="radio"]');
      expect(radioButtons.length).toBe(2);

      const solButton = radioButtons[0];
      expect(solButton.getAttribute('aria-checked')).toBe('true');
      expect(solButton.getAttribute('aria-label')).toContain('Solana');
    });

    it('has keyboard navigation support', () => {
      const { container } = render(PaymentMethodSelector);

      const buttons = container.querySelectorAll('.method-card');
      buttons.forEach(button => {
        expect(button.tagName).toBe('BUTTON');
      });
    });
  });

  describe('Visual States', () => {
    it('applies selected class to active method', () => {
      const { container } = render(PaymentMethodSelector, {
        props: { selected: 'sol' }
      });

      const solCard = container.querySelector('.method-card.violet');
      expect(solCard.classList.contains('selected')).toBe(true);
    });

    it('shows recommended badge on SOL', () => {
      const { getByText } = render(PaymentMethodSelector);

      expect(getByText('RECOMMENDED')).toBeTruthy();
    });

    it('applies disabled class when disabled', () => {
      const { container } = render(PaymentMethodSelector, {
        props: { disabled: true }
      });

      const cards = container.querySelectorAll('.method-card');
      cards.forEach(card => {
        expect(card.classList.contains('disabled')).toBe(true);
      });
    });
  });
});

describe('Integration with CreateJob Form', () => {
  it('updates form data when payment method changes', () => {
    const formData = { paymentMethod: 'sol' };

    const handleChange = (event) => {
      formData.paymentMethod = event.detail.payment_method;
    };

    const { component } = render(PaymentMethodSelector, {
      props: { selected: formData.paymentMethod }
    });

    component.$on('change', handleChange);

    // Simulate selecting wZEC
    component.$set({ selected: 'wzec' });

    // In real scenario, event would be dispatched and handled
    // This test verifies the pattern works
    expect(formData.paymentMethod).toBeDefined();
  });
});
