package dev.dov.mk;

public interface CompareOp {
    int NONE = 0;
    int NEVER = 1;
    int LESS = 2;
    int EQUAL = 3;
    int LESS_EQUAL = 4;
    int GREATER = 5;
    int NOT_EQUAL = 6;
    int GREATER_EQUAL = 7;
    int ALWAYS = 8;
}
