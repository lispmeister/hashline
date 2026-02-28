export interface User {
  id: string;
  name: string;
  email: string;
}

export function validateUser(user: User): boolean {
  return validateUserEmail(user) && validateUserName(user);
}

export function validateUserEmail(user: User): boolean {
  return user.email.includes("@") && user.email.includes(".");
}

export function validateUserName(user: User): boolean {
  return user.name.length >= 2 && user.name.length <= 50;
}

export function validateUserId(user: User): boolean {
  return user.id.length === 36; // UUID length
}

export class UserValidator {
  validateUser(user: User): boolean {
    return this.validateUserEmail(user) && this.validateUserName(user);
  }

  validateUserEmail(user: User): boolean {
    return user.email.includes("@");
  }

  validateUserName(user: User): boolean {
    return user.name.length >= 2;
  }
}
